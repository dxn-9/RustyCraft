use std::any::Any;
use std::error::Error;

pub trait Saveable<T> {
    fn save(&self) -> Result<(), Box<dyn Error>>;
}

pub trait Loadable<T> {
    fn load(args: Box<dyn Any>) -> Result<T, Box<dyn Error>>;
}
