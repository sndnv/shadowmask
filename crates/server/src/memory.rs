use std::path::Path;

pub fn dir_size_bytes(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter_map(|entry| entry.metadata().ok())
        .filter(|meta| meta.is_file())
        .map(|meta| meta.len())
        .sum()
}

fn parse_cgroup_v2_max(contents: &str) -> Option<u64> {
    let trimmed = contents.trim();
    if trimmed == "max" {
        return None;
    }
    trimmed.parse().ok()
}

fn parse_cgroup_v1_limit(contents: &str) -> Option<u64> {
    let value: u64 = contents.trim().parse().ok()?;
    if value >= 1u64 << 62 {
        None
    } else {
        Some(value)
    }
}

fn parse_mem_available(contents: &str) -> Option<u64> {
    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            let kib: u64 = rest.trim().trim_end_matches("kB").trim().parse().ok()?;
            return Some(kib * 1024);
        }
    }
    None
}

fn cgroup_indicates_container(contents: &str) -> bool {
    ["docker", "kubepods", "containerd", "lxc", "libpod"]
        .iter()
        .any(|marker| contents.contains(marker))
}

pub fn memory_warning(
    kind: &str,
    model_dir: &str,
    model_bytes: u64,
    budget_bytes: u64,
) -> Option<String> {
    if model_bytes == 0 {
        return None;
    }
    if budget_bytes == 0 {
        return Some(format!(
            "{kind}: could not determine the available memory budget; model [{model_dir}] is {} and may be killed (OOM) on a memory-constrained host",
            human_bytes(model_bytes)
        ));
    }
    if model_bytes.saturating_mul(12) / 10 > budget_bytes {
        return Some(format!(
            "{kind}: model [{model_dir}] is {} but only {} of memory is available; {kind} jobs may be killed (OOM)",
            human_bytes(model_bytes),
            human_bytes(budget_bytes)
        ));
    }
    None
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

#[cfg(feature = "enrichment")]
pub fn available_memory_bytes() -> u64 {
    if in_container() {
        cgroup_memory_limit()
            .or_else(host_available_memory)
            .unwrap_or(0)
    } else {
        host_available_memory().unwrap_or(0)
    }
}

#[cfg(feature = "enrichment")]
fn in_container() -> bool {
    Path::new("/.dockerenv").exists()
        || Path::new("/run/.containerenv").exists()
        || std::fs::read_to_string("/proc/1/cgroup")
            .map(|contents| cgroup_indicates_container(&contents))
            .unwrap_or(false)
}

#[cfg(feature = "enrichment")]
fn cgroup_memory_limit() -> Option<u64> {
    if let Ok(contents) = std::fs::read_to_string("/sys/fs/cgroup/memory.max") {
        return parse_cgroup_v2_max(&contents);
    }
    std::fs::read_to_string("/sys/fs/cgroup/memory/memory.limit_in_bytes")
        .ok()
        .and_then(|contents| parse_cgroup_v1_limit(&contents))
}

#[cfg(feature = "enrichment")]
fn host_available_memory() -> Option<u64> {
    std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|contents| parse_mem_available(&contents))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn dir_size_sums_files_only() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("model.bin"), vec![0u8; 2048]).unwrap();
        fs::write(dir.path().join("config.json"), vec![0u8; 100]).unwrap();
        fs::create_dir(dir.path().join("nested")).unwrap();
        assert_eq!(dir_size_bytes(dir.path()), 2148);
        assert_eq!(dir_size_bytes(&dir.path().join("missing")), 0);
    }

    #[test]
    fn cgroup_v2_max_parsing() {
        assert_eq!(parse_cgroup_v2_max("max\n"), None);
        assert_eq!(parse_cgroup_v2_max("8589934592\n"), Some(8_589_934_592));
        assert_eq!(parse_cgroup_v2_max("garbage"), None);
    }

    #[test]
    fn cgroup_v1_limit_parsing() {
        assert_eq!(parse_cgroup_v1_limit("8589934592"), Some(8_589_934_592));
        assert_eq!(parse_cgroup_v1_limit("9223372036854771712"), None);
        assert_eq!(parse_cgroup_v1_limit("nope"), None);
    }

    #[test]
    fn mem_available_parsing() {
        let meminfo = "MemTotal:       16000000 kB\nMemAvailable:    8000000 kB\n";
        assert_eq!(parse_mem_available(meminfo), Some(8_000_000 * 1024));
        assert_eq!(parse_mem_available("MemTotal: 1 kB\n"), None);
    }

    #[test]
    fn container_markers_detected() {
        assert!(cgroup_indicates_container("0::/docker/abc123def456\n"));
        assert!(cgroup_indicates_container("12:memory:/kubepods/pod\n"));
        assert!(!cgroup_indicates_container("0::/init.scope\n"));
    }

    #[test]
    fn warning_fires_when_budget_unknown() {
        let warning = memory_warning("transcription", "/models/x", 1000, 0).unwrap();
        assert!(warning.contains("could not determine"));
    }

    #[test]
    fn warning_fires_when_model_exceeds_budget() {
        let warning =
            memory_warning("translation", "/models/big", 12_000_000_000, 8_000_000_000).unwrap();
        assert!(warning.contains("may be killed (OOM)"));
        assert!(warning.contains("GiB"));
    }

    #[test]
    fn no_warning_when_model_fits() {
        assert!(
            memory_warning(
                "transcription",
                "/models/small",
                3_000_000_000,
                16_000_000_000
            )
            .is_none()
        );
    }

    #[test]
    fn no_warning_when_no_model() {
        assert!(memory_warning("transcription", "/models/x", 0, 0).is_none());
    }
}
