#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubtitleDelivery {
    HlsVtt,
    Burned,
    Embedded,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AudioDelivery {
    Embedded,
    Stream,
}
