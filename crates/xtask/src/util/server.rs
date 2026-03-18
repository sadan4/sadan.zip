use clap::ValueEnum;

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum ServerTarget {
    Napi,
    Js,
    Native,
}