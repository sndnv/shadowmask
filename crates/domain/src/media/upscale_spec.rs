#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UpscaleSpec {
    pub source_path: String,
    pub target_height: u32,
    pub output_path: String,
}
