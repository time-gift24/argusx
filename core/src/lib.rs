pub enum ResponseEvent {
    Done(Option<Usage>),
    Error(Error),
}

pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

pub struct Error {
    pub message: String,
}
