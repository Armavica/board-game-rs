use std::ptr::null_mut;

use crate::bindings::{cudnnActivationDescriptor_t, cudnnActivationMode_t, cudnnConvolutionDescriptor_t, cudnnConvolutionMode_t, cudnnCreateActivationDescriptor, cudnnCreateConvolutionDescriptor, cudnnCreateFilterDescriptor, cudnnCreatePoolingDescriptor, cudnnCreateTensorDescriptor, cudnnDataType_t, cudnnDestroyActivationDescriptor, cudnnDestroyConvolutionDescriptor, cudnnDestroyFilterDescriptor, cudnnDestroyPoolingDescriptor, cudnnDestroyTensorDescriptor, cudnnFilterDescriptor_t, cudnnGetFilterSizeInBytes, cudnnGetTensorSizeInBytes, cudnnNanPropagation_t, cudnnPoolingDescriptor_t, cudnnPoolingMode_t, cudnnSetActivationDescriptor, cudnnSetConvolution2dDescriptor, cudnnSetFilter4dDescriptor, cudnnSetPooling2dDescriptor, cudnnSetTensor4dDescriptor, cudnnTensorDescriptor_t, cudnnTensorFormat_t, cudnnGetPooling2dForwardOutputDim, cudnnGetConvolution2dForwardOutputDim};
use crate::wrapper::status::Status;

pub struct TensorDescriptor(cudnnTensorDescriptor_t);

impl Drop for TensorDescriptor {
    fn drop(&mut self) {
        unsafe { cudnnDestroyTensorDescriptor(self.0).unwrap() }
    }
}

impl TensorDescriptor {
    pub fn new(n: i32, c: i32, h: i32, w: i32, data_type: cudnnDataType_t, format: cudnnTensorFormat_t) -> Self {
        unsafe {
            let mut inner = null_mut();
            cudnnCreateTensorDescriptor(&mut inner as *mut _).unwrap();
            cudnnSetTensor4dDescriptor(
                inner,
                format,
                data_type,
                n, c, h, w,
            ).unwrap();
            TensorDescriptor(inner)
        }
    }

    pub unsafe fn inner(&self) -> cudnnTensorDescriptor_t {
        self.0
    }

    pub fn size(&self) -> usize {
        unsafe {
            let mut result = 0;
            cudnnGetTensorSizeInBytes(self.0, &mut result as *mut _).unwrap();
            result
        }
    }
}

pub struct FilterDescriptor(cudnnFilterDescriptor_t);

impl Drop for FilterDescriptor {
    fn drop(&mut self) { unsafe { cudnnDestroyFilterDescriptor(self.0).unwrap() } }
}

impl FilterDescriptor {
    /// * `k`: output channels
    /// * `c`: input channels
    /// * `(h, w)`: kernel size
    pub fn new(k: i32, c: i32, h: i32, w: i32, data_type: cudnnDataType_t, format: cudnnTensorFormat_t) -> Self {
        unsafe {
            let mut inner = null_mut();
            cudnnCreateFilterDescriptor(&mut inner as *mut _).unwrap();
            cudnnSetFilter4dDescriptor(
                inner,
                data_type,
                format,
                k, c, h, w,
            ).unwrap();
            FilterDescriptor(inner)
        }
    }

    pub unsafe fn inner(&self) -> cudnnFilterDescriptor_t {
        self.0
    }

    pub fn size(&self) -> usize {
        unsafe {
            let mut result = 0;
            cudnnGetFilterSizeInBytes(self.0, &mut result as *mut _).unwrap();
            result
        }
    }
}

pub struct ConvolutionDescriptor(cudnnConvolutionDescriptor_t);

impl Drop for ConvolutionDescriptor {
    fn drop(&mut self) {
        unsafe { cudnnDestroyConvolutionDescriptor(self.0).unwrap() }
    }
}

impl ConvolutionDescriptor {
    pub fn new(
        pad_h: i32,
        pad_w: i32,
        stride_h: i32,
        stride_v: i32,
        dilation_h: i32,
        dilation_w: i32,
        data_type: cudnnDataType_t,
    ) -> Self {
        unsafe {
            let mut inner = null_mut();
            cudnnCreateConvolutionDescriptor(&mut inner as *mut _).unwrap();
            cudnnSetConvolution2dDescriptor(
                inner,
                pad_h, pad_w, stride_h, stride_v, dilation_h, dilation_w,
                cudnnConvolutionMode_t::CUDNN_CROSS_CORRELATION, data_type,
            ).unwrap();
            ConvolutionDescriptor(inner)
        }
    }

    pub fn output_shape(&self, input_desc: &TensorDescriptor, filter_desc: &FilterDescriptor) -> [i32; 4] {
        unsafe {
            let mut n = 0;
            let mut c = 0;
            let mut h = 0;
            let mut w = 0;
            cudnnGetConvolution2dForwardOutputDim(
                self.inner(),
                input_desc.inner(),
                filter_desc.inner(),
                &mut n as *mut _,
                &mut c as *mut _,
                &mut h as *mut _,
                &mut w as *mut _,
            ).unwrap();
            [n, c, h, w]
        }
    }

    pub unsafe fn inner(&self) -> cudnnConvolutionDescriptor_t {
        self.0
    }
}

pub struct ActivationDescriptor(cudnnActivationDescriptor_t);

impl Drop for ActivationDescriptor {
    fn drop(&mut self) {
        unsafe { cudnnDestroyActivationDescriptor(self.0).unwrap() }
    }
}

impl ActivationDescriptor {
    pub fn new(
        mode: cudnnActivationMode_t,
        coef: f32,
    ) -> Self {
        unsafe {
            let mut inner = null_mut();
            cudnnCreateActivationDescriptor(&mut inner as *mut _).unwrap();
            cudnnSetActivationDescriptor(
                inner,
                mode,
                cudnnNanPropagation_t::CUDNN_PROPAGATE_NAN,
                coef as f64,
            ).unwrap();
            ActivationDescriptor(inner)
        }
    }

    pub unsafe fn inner(&self) -> cudnnActivationDescriptor_t {
        self.0
    }
}

pub struct PoolingDescriptor(cudnnPoolingDescriptor_t);

impl Drop for PoolingDescriptor {
    fn drop(&mut self) {
        unsafe { cudnnDestroyPoolingDescriptor(self.0).unwrap() }
    }
}

impl PoolingDescriptor {
    pub fn new(
        mode: cudnnPoolingMode_t,
        h: i32,
        w: i32,
        pad_h: i32,
        pad_w: i32,
        stride_h: i32,
        stride_v: i32,
    ) -> Self {
        unsafe {
            let mut inner = null_mut();
            cudnnCreatePoolingDescriptor(&mut inner as *mut _).unwrap();
            cudnnSetPooling2dDescriptor(
                inner,
                mode,
                cudnnNanPropagation_t::CUDNN_PROPAGATE_NAN,
                h, w, pad_h, pad_w, stride_h, stride_v,
            ).unwrap();
            PoolingDescriptor(inner)
        }
    }

    pub fn output_shape(&self, input_desc: &TensorDescriptor) -> [i32; 4] {
        unsafe {
            let mut n = 0;
            let mut c = 0;
            let mut h = 0;
            let mut w = 0;
            cudnnGetPooling2dForwardOutputDim(
                self.inner(),
                input_desc.inner(),
                &mut n as *mut _,
                &mut c as *mut _,
                &mut h as *mut _,
                &mut w as *mut _,
            ).unwrap();
            [n, c, h, w]
        }
    }

    pub unsafe fn inner(&self) -> cudnnPoolingDescriptor_t {
        self.0
    }
}
