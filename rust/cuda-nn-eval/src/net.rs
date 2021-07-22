use bytemuck::{cast_slice, cast_slice_mut};
use itertools::Itertools;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use cuda_sys::bindings::{cudnnActivationMode_t, cudnnConvolutionFwdAlgo_t, cudnnDataType_t, cudnnTensorFormat_t};
use cuda_sys::wrapper::descriptor::{ActivationDescriptor, ConvolutionDescriptor, FilterDescriptor, TensorDescriptor};
use cuda_sys::wrapper::handle::CudnnHandle;
use cuda_sys::wrapper::mem::DeviceMem;
use cuda_sys::wrapper::operation::{find_conv_algorithms, run_activation_in_place, run_add_tensor, run_conv};

#[derive(Copy, Clone)]
pub struct NetDefinition {
    pub tower_depth: usize,
    pub channels: i32,
}

struct Layer {
    filter_mem: DeviceMem,
    bias_mem: DeviceMem,
}

pub struct NetEvaluator {
    handle: CudnnHandle,
    batch_size: i32,

    def: NetDefinition,
    layers: Vec<Layer>,

    image_desc: TensorDescriptor,

    conv_desc: ConvolutionDescriptor,
    filter_desc: FilterDescriptor,
    bias_desc: TensorDescriptor,
    act_desc: ActivationDescriptor,

    algo: cudnnConvolutionFwdAlgo_t,
    work_mem: DeviceMem,
}

impl NetEvaluator {
    pub fn new(mut handle: CudnnHandle, def: NetDefinition, batch_size: i32) -> Self {
        let image_desc = TensorDescriptor::new(
            batch_size, def.channels, 7, 7,
            cudnnDataType_t::CUDNN_DATA_FLOAT, cudnnTensorFormat_t::CUDNN_TENSOR_NCHW,
        );
        let bias_desc = TensorDescriptor::new(
            1, def.channels, 1, 1,
            cudnnDataType_t::CUDNN_DATA_FLOAT, cudnnTensorFormat_t::CUDNN_TENSOR_NCHW,
        );
        let filter_desc = FilterDescriptor::new(
            def.channels, def.channels, 3, 3,
            cudnnDataType_t::CUDNN_DATA_FLOAT, cudnnTensorFormat_t::CUDNN_TENSOR_NCHW,
        );
        let conv_desc = ConvolutionDescriptor::new(
            1, 1, 1, 1, 1, 1, cudnnDataType_t::CUDNN_DATA_FLOAT,
        );
        let act_desc = ActivationDescriptor::new(cudnnActivationMode_t::CUDNN_ACTIVATION_RELU, 0.0);

        let mut rng = StdRng::seed_from_u64(0);

        let layers = (0..def.tower_depth).map(|_| {
            let mut filter_mem = DeviceMem::alloc(filter_desc.size());
            let mut bias_mem = DeviceMem::alloc(bias_desc.size());

            filter_mem.copy_from_host(
                cast_slice(
                    &(0..filter_mem.size() / 4)
                        .map(|_| rng.gen::<f32>())
                        .collect_vec()
                )
            );
            bias_mem.copy_from_host(
                cast_slice(
                    &(0..bias_mem.size() / 4)
                        .map(|_| rng.gen::<f32>())
                        .collect_vec()
                )
            );

            Layer {
                filter_mem,
                bias_mem,
            }
        }).collect();

        let algo_info = find_conv_algorithms(
            &mut handle,
            &conv_desc, &image_desc, &filter_desc, &image_desc,
        )[0];

        let workspace = DeviceMem::alloc(algo_info.memory);

        NetEvaluator {
            handle,
            batch_size,
            def,
            layers,
            image_desc,
            conv_desc,
            filter_desc,
            bias_desc,
            act_desc,
            algo: algo_info.algo,
            work_mem: workspace,
        }
    }

    /// Runs `data = net(data)`.
    pub fn eval(&mut self, data: &mut Vec<f32>) {
        assert_eq!(self.batch_size * self.def.channels * 7 * 7, data.len() as i32);

        let mut prev = DeviceMem::alloc(self.image_desc.size());
        let mut next = DeviceMem::alloc(self.image_desc.size());

        prev.copy_from_host(cast_slice(data));

        for layer in &self.layers {
            let Layer { filter_mem, bias_mem } = layer;

            // next = conv(prev, filter)
            run_conv(
                &mut self.handle,
                &self.conv_desc,
                self.algo,
                &mut self.work_mem,
                &self.image_desc,
                &mut next,
                &self.image_desc,
                &prev,
                &self.filter_desc,
                filter_mem,
            );

            // next += bias
            run_add_tensor(
                &mut self.handle,
                &self.image_desc,
                &mut next,
                &self.bias_desc,
                bias_mem,
            );

            // next = act(next)
            run_activation_in_place(
                &mut self.handle,
                &self.act_desc,
                &self.image_desc,
                &mut next,
            );

            // swap for next iteration
            std::mem::swap(&mut next, &mut prev);
        }

        // copy output back
        prev.copy_to_host(cast_slice_mut(data));
    }
}