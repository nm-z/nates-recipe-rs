pub fn banned() -> recipe::LayerSpec {
	recipe::LayerSpec::Convolution {
		filters: 12,
		kernel: 3,
		stride: 2,
		activation: recipe::Activation::Relu,
	}
}
