pub trait Uniform {
    type Uniform;

    fn to_uniform(&self) -> Self::Uniform;
}
