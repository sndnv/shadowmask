use std::num::NonZero;
use std::path::Path;
use std::time::{Duration, Instant};

use domain::diagnostics::{
    AccelerationMode, CapabilityProbe, CapabilityReport, FfmpegReport, HardwareReport,
    HardwareTest, HostReport, Presence,
};
use domain::process::ProcessSpawner;
use sysinfo::{CpuRefreshKind, RefreshKind, System};

use crate::transcode::TokioProcessSpawner;

const BINARY: &str = "ffmpeg";
const ENCODERS: [&str; 2] = ["libx264", "h264_vaapi"];
const FILTERS: [&str; 5] = ["zscale", "tonemap", "subtitles", "scale_vaapi", "tonemap_vaapi"];
const TEST_SOURCE: &str = "testsrc2=size=1280x720:rate=30:duration=1";
const RUN_LIMIT: Duration = Duration::from_secs(30);

pub struct FfmpegCapabilityProbe<S = TokioProcessSpawner> {
    spawner: S,
    mode: AccelerationMode,
    device: String,
    in_use: Option<String>,
}

impl FfmpegCapabilityProbe {
    pub fn new(mode: AccelerationMode, device: impl Into<String>, in_use: Option<String>) -> Self {
        Self::with_spawner(TokioProcessSpawner, mode, device, in_use)
    }
}

impl<S: ProcessSpawner> FfmpegCapabilityProbe<S> {
    pub fn with_spawner(
        spawner: S,
        mode: AccelerationMode,
        device: impl Into<String>,
        in_use: Option<String>,
    ) -> Self {
        Self { spawner, mode, device: device.into(), in_use }
    }

    async fn run(&self, args: &[&str]) -> Result<String, String> {
        let args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
        let output = tokio::time::timeout(RUN_LIMIT, self.spawner.run_captured(BINARY, &args))
            .await
            .map_err(|_| format!("ffmpeg did not finish within {} s", RUN_LIMIT.as_secs()))?
            .map_err(|error| error.to_string())?;
        if output.success { Ok(output.stdout) } else { Err(output.failure_detail(5)) }
    }

    async fn ffmpeg(&self) -> FfmpegReport {
        let version = match self.run(&["-hide_banner", "-version"]).await {
            Ok(text) => parse_version(&text),
            Err(error) => {
                return FfmpegReport {
                    version: None,
                    error: Some(error),
                    hwaccels: Vec::new(),
                    encoders: presence(&ENCODERS, ""),
                    filters: presence(&FILTERS, ""),
                };
            }
        };
        let hwaccels = self.run(&["-hide_banner", "-hwaccels"]).await.unwrap_or_default();
        let encoders = self.run(&["-hide_banner", "-encoders"]).await.unwrap_or_default();
        let filters = self.run(&["-hide_banner", "-filters"]).await.unwrap_or_default();
        FfmpegReport {
            version,
            error: None,
            hwaccels: parse_hwaccels(&hwaccels),
            encoders: presence(&ENCODERS, &encoders),
            filters: presence(&FILTERS, &filters),
        }
    }

    async fn hardware(&self, ffmpeg: &FfmpegReport) -> HardwareReport {
        let test = match &self.in_use {
            None if self.mode == AccelerationMode::Off => {
                HardwareTest::Skipped { reason: "hardware acceleration is off".to_owned() }
            }
            None => HardwareTest::Skipped { reason: format!("no render node at {}", self.device) },
            Some(_) if !ffmpeg.has_encoder("h264_vaapi") => {
                HardwareTest::Skipped { reason: "this ffmpeg has no h264_vaapi encoder".to_owned() }
            }
            Some(device) => self.test_encode(device).await,
        };
        HardwareReport {
            mode: self.mode,
            device: self.device.clone(),
            device_present: Path::new(&self.device).exists(),
            in_use: self.in_use.is_some(),
            test,
        }
    }

    async fn test_encode(&self, device: &str) -> HardwareTest {
        let detail = match self.timed_encode(device, false).await {
            Ok(elapsed_ms) => return HardwareTest::Works { low_power: false, elapsed_ms },
            Err(detail) => detail,
        };
        match self.timed_encode(device, true).await {
            Ok(elapsed_ms) => HardwareTest::Works { low_power: true, elapsed_ms },
            Err(low_power_detail) => HardwareTest::Failed { detail, low_power_detail },
        }
    }

    async fn timed_encode(&self, device: &str, low_power: bool) -> Result<u64, String> {
        let mut args = vec![
            "-hide_banner",
            "-v",
            "error",
            "-vaapi_device",
            device,
            "-f",
            "lavfi",
            "-i",
            TEST_SOURCE,
            "-vf",
            "format=nv12,hwupload",
            "-c:v",
            "h264_vaapi",
        ];
        if low_power {
            args.extend(["-low_power", "1"]);
        }
        args.extend(["-f", "null", "-"]);
        let started = Instant::now();
        self.run(&args).await?;
        Ok(u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX))
    }
}

impl<S: ProcessSpawner> CapabilityProbe for FfmpegCapabilityProbe<S> {
    async fn probe(&self) -> CapabilityReport {
        let ffmpeg = self.ffmpeg().await;
        let hardware = self.hardware(&ffmpeg).await;
        CapabilityReport { host: host(), ffmpeg, hardware }
    }
}

fn host() -> HostReport {
    let system =
        System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing()));
    let (brand, vendor) =
        system.cpus().first().map_or(("", ""), |cpu| (cpu.brand(), cpu.vendor_id()));
    let cpu = cpu_name(brand, vendor, &System::cpu_arch());
    let threads = std::thread::available_parallelism().map_or(1, NonZero::get);
    HostReport { cpu, cores: System::physical_core_count(), threads }
}

fn cpu_name(brand: &str, vendor: &str, arch: &str) -> Option<String> {
    match (brand.trim(), vendor.trim(), arch.trim()) {
        ("", "", "") => None,
        ("", "", arch) => Some(arch.to_owned()),
        ("", vendor, "") => Some(vendor.to_owned()),
        ("", vendor, arch) => Some(format!("{vendor} ({arch})")),
        (brand, _, _) => Some(brand.to_owned()),
    }
}

fn parse_version(text: &str) -> Option<String> {
    let line = text.lines().next()?;
    let rest = line.strip_prefix("ffmpeg version ")?;
    rest.split_whitespace().next().map(str::to_owned)
}

fn parse_hwaccels(text: &str) -> Vec<String> {
    text.lines()
        .skip_while(|line| !line.starts_with("Hardware acceleration methods"))
        .skip(1)
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

fn presence(names: &[&str], listing: &str) -> Vec<Presence> {
    names
        .iter()
        .map(|name| Presence {
            name: (*name).to_owned(),
            present: listing.lines().any(|line| line.split_whitespace().nth(1) == Some(*name)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::Mutex;

    use domain::process::CommandOutput;

    use super::*;

    const VERSION: &str = "ffmpeg version 5.1.6-0+deb12u1 Copyright (c) 2000-2024 the FFmpeg developers\nbuilt with gcc 12\n";
    const HWACCELS: &str = "Hardware acceleration methods:\nvdpau\nvaapi\n\n";
    const ENCODER_LIST: &str = "Encoders:\n V..... = Video\n ------\n V....D libx264              libx264 H.264 / AVC\n V....D h264_vaapi           H.264/AVC (VAAPI)\n";
    const FILTER_LIST: &str = "Filters:\n  T.. = Timeline support\n ... zscale            V->V       Apply resizing\n ... tonemap           V->V       Conversion to/from different dynamic ranges.\n ... subtitles         V->V       Render text subtitles\n";

    #[derive(Default)]
    struct Canned {
        missing: bool,
        hardware_fails: bool,
        low_power_fails: bool,
        encode_hangs: bool,
        calls: Mutex<Vec<Vec<String>>>,
    }

    impl Canned {
        fn reply(&self, args: &[String]) -> CommandOutput {
            let stdout = match args.get(1).map(String::as_str) {
                Some("-version") => VERSION,
                Some("-hwaccels") => HWACCELS,
                Some("-encoders") => ENCODER_LIST,
                Some("-filters") => FILTER_LIST,
                _ => "",
            };
            let failing = if args.contains(&"-low_power".to_owned()) {
                self.low_power_fails
            } else {
                self.hardware_fails && args.contains(&"h264_vaapi".to_owned())
            };
            CommandOutput {
                success: !failing,
                stdout: stdout.to_owned(),
                stderr: if failing {
                    "No usable encoding entrypoint".to_owned()
                } else {
                    String::new()
                },
                status: String::new(),
            }
        }

        fn encodes(&self) -> usize {
            let calls = self.calls.lock().unwrap();
            calls.iter().filter(|args| args.contains(&"h264_vaapi".to_owned())).count()
        }
    }

    impl ProcessSpawner for Canned {
        async fn run(&self, _program: &str, _args: &[String]) -> io::Result<bool> {
            unreachable!("the probe always captures output")
        }

        async fn run_captured(&self, program: &str, args: &[String]) -> io::Result<CommandOutput> {
            assert_eq!(program, "ffmpeg");
            self.calls.lock().unwrap().push(args.to_vec());
            if self.missing {
                return Err(io::Error::new(io::ErrorKind::NotFound, "No such file or directory"));
            }
            if self.encode_hangs && args.contains(&"h264_vaapi".to_owned()) {
                std::future::pending::<()>().await;
            }
            Ok(self.reply(args))
        }
    }

    fn probe(
        spawner: Canned,
        mode: AccelerationMode,
        in_use: Option<&str>,
    ) -> FfmpegCapabilityProbe<Canned> {
        FfmpegCapabilityProbe::with_spawner(
            spawner,
            mode,
            "/dev/dri/renderD128",
            in_use.map(str::to_owned),
        )
    }

    fn names(presences: &[Presence]) -> Vec<(&str, bool)> {
        presences.iter().map(|presence| (presence.name.as_str(), presence.present)).collect()
    }

    #[tokio::test]
    async fn the_report_reads_ffmpeg_and_the_host() {
        let report = probe(Canned::default(), AccelerationMode::Off, None).probe().await;

        assert_eq!(report.ffmpeg.version.as_deref(), Some("5.1.6-0+deb12u1"));
        assert_eq!(report.ffmpeg.error, None);
        assert_eq!(report.ffmpeg.hwaccels, ["vdpau", "vaapi"]);
        assert_eq!(names(&report.ffmpeg.encoders), [("libx264", true), ("h264_vaapi", true)]);
        assert_eq!(
            names(&report.ffmpeg.filters),
            [
                ("zscale", true),
                ("tonemap", true),
                ("subtitles", true),
                ("scale_vaapi", false),
                ("tonemap_vaapi", false)
            ]
        );
        assert!(report.host.threads >= 1);
        assert!(report.host.cpu.is_some());
        assert_eq!(report.hardware.mode, AccelerationMode::Off);
        assert_eq!(report.hardware.device, "/dev/dri/renderD128");
        assert!(!report.hardware.in_use);
    }

    #[tokio::test]
    async fn the_test_encode_is_skipped_without_a_device_in_use() {
        let off = probe(Canned::default(), AccelerationMode::Off, None).probe().await;
        let absent = probe(Canned::default(), AccelerationMode::Auto, None).probe().await;

        assert_eq!(
            off.hardware.test,
            HardwareTest::Skipped { reason: "hardware acceleration is off".to_owned() }
        );
        assert_eq!(
            absent.hardware.test,
            HardwareTest::Skipped { reason: "no render node at /dev/dri/renderD128".to_owned() }
        );
    }

    #[tokio::test]
    async fn a_working_device_is_tested_once() {
        let canned = Canned::default();
        let probe = probe(canned, AccelerationMode::Auto, Some("/dev/dri/renderD128"));

        let report = probe.probe().await;

        assert!(matches!(report.hardware.test, HardwareTest::Works { low_power: false, .. }));
        assert!(report.hardware.in_use);
        assert_eq!(probe.spawner.encodes(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_test_encode_that_never_finishes_fails_at_its_limit() {
        let canned = Canned { encode_hangs: true, ..Canned::default() };
        let probe = probe(canned, AccelerationMode::Vaapi, Some("/dev/dri/renderD128"));

        let report = probe.probe().await;

        let limit = "ffmpeg did not finish within 30 s".to_owned();
        assert_eq!(
            report.hardware.test,
            HardwareTest::Failed { detail: limit.clone(), low_power_detail: limit }
        );
        assert_eq!(report.ffmpeg.version.as_deref(), Some("5.1.6-0+deb12u1"));
    }

    #[tokio::test]
    async fn a_device_that_only_encodes_in_low_power_says_so() {
        let canned = Canned { hardware_fails: true, ..Canned::default() };
        let probe = probe(canned, AccelerationMode::Vaapi, Some("/dev/dri/renderD128"));

        let report = probe.probe().await;

        assert!(matches!(report.hardware.test, HardwareTest::Works { low_power: true, .. }));
        assert_eq!(probe.spawner.encodes(), 2);
    }

    #[tokio::test]
    async fn a_device_that_cannot_encode_reports_both_errors() {
        let canned = Canned { hardware_fails: true, low_power_fails: true, ..Canned::default() };

        let report =
            probe(canned, AccelerationMode::Vaapi, Some("/dev/dri/renderD128")).probe().await;

        assert_eq!(
            report.hardware.test,
            HardwareTest::Failed {
                detail: "No usable encoding entrypoint".to_owned(),
                low_power_detail: "No usable encoding entrypoint".to_owned(),
            }
        );
    }

    #[tokio::test]
    async fn a_missing_ffmpeg_is_reported_and_nothing_is_tested() {
        let canned = Canned { missing: true, ..Canned::default() };

        let report =
            probe(canned, AccelerationMode::Auto, Some("/dev/dri/renderD128")).probe().await;

        assert_eq!(report.ffmpeg.version, None);
        assert_eq!(report.ffmpeg.error.as_deref(), Some("No such file or directory"));
        assert!(report.ffmpeg.encoders.iter().all(|encoder| !encoder.present));
        assert_eq!(
            report.hardware.test,
            HardwareTest::Skipped { reason: "this ffmpeg has no h264_vaapi encoder".to_owned() }
        );
    }

    #[test]
    fn a_version_line_in_another_shape_reads_as_unknown() {
        assert_eq!(parse_version("ffmpeg version n7.0 Copyright"), Some("n7.0".to_owned()));
        assert_eq!(parse_version("something else"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn hardware_methods_need_their_heading() {
        assert!(parse_hwaccels("vaapi\n").is_empty());
    }

    #[test]
    fn a_cpu_without_a_brand_is_named_by_its_vendor_and_architecture() {
        assert_eq!(
            cpu_name(" 12th Gen Intel(R) Core(TM) i3-1220P ", "GenuineIntel", "x86_64").as_deref(),
            Some("12th Gen Intel(R) Core(TM) i3-1220P")
        );
        assert_eq!(cpu_name("", "Apple", "aarch64").as_deref(), Some("Apple (aarch64)"));
        assert_eq!(cpu_name(" ", "", "aarch64").as_deref(), Some("aarch64"));
        assert_eq!(cpu_name("", "Apple", " ").as_deref(), Some("Apple"));
        assert_eq!(cpu_name("", " ", ""), None);
    }
}
