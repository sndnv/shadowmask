use ::api::Capability;

pub struct CapabilityInputs {
    pub transcription: bool,
    pub translation: bool,
    pub upscaling: bool,
    pub opensubtitles: bool,
    pub tmdb: bool,
    pub tls: bool,
    pub webhooks: bool,
    pub content_fetch: bool,
    pub hardware_transcode_available: bool,
    pub hardware_transcode_enabled: bool,
}

pub fn select_capabilities(ml: bool, inputs: CapabilityInputs) -> Vec<Capability> {
    vec![
        Capability::new("transcription", ml, ml && inputs.transcription),
        Capability::new("translation", ml, ml && inputs.translation),
        Capability::new("upscaling", true, inputs.upscaling),
        Capability::new("opensubtitles", true, inputs.opensubtitles),
        Capability::new("tmdb", true, inputs.tmdb),
        Capability::new("tls", true, inputs.tls),
        Capability::new("webhooks", true, inputs.webhooks),
        Capability::new("content_fetch", true, inputs.content_fetch),
        Capability::new(
            "hardware_transcode",
            inputs.hardware_transcode_available,
            inputs.hardware_transcode_enabled,
        ),
    ]
}

pub fn server_capabilities(inputs: CapabilityInputs) -> Vec<Capability> {
    select_capabilities(cfg!(feature = "enrichment"), inputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_on() -> CapabilityInputs {
        CapabilityInputs {
            transcription: true,
            translation: true,
            upscaling: true,
            opensubtitles: true,
            tmdb: true,
            tls: true,
            webhooks: true,
            content_fetch: true,
            hardware_transcode_available: true,
            hardware_transcode_enabled: true,
        }
    }

    fn find<'a>(caps: &'a [Capability], name: &str) -> &'a Capability {
        caps.iter().find(|c| c.name == name).expect("capability")
    }

    #[test]
    fn compiled_marks_ml_available_and_others_track_inputs() {
        let caps = select_capabilities(true, all_on());
        assert_eq!(caps.len(), 9);
        assert_eq!(
            find(&caps, "transcription"),
            &Capability::new("transcription", true, true)
        );
        assert_eq!(
            find(&caps, "translation"),
            &Capability::new("translation", true, true)
        );
        assert_eq!(
            find(&caps, "upscaling"),
            &Capability::new("upscaling", true, true)
        );
        assert_eq!(
            find(&caps, "opensubtitles"),
            &Capability::new("opensubtitles", true, true)
        );
        assert_eq!(find(&caps, "tmdb"), &Capability::new("tmdb", true, true));
        assert_eq!(find(&caps, "tls"), &Capability::new("tls", true, true));
        assert_eq!(
            find(&caps, "webhooks"),
            &Capability::new("webhooks", true, true)
        );
        assert_eq!(
            find(&caps, "content_fetch"),
            &Capability::new("content_fetch", true, true)
        );
        assert_eq!(
            find(&caps, "hardware_transcode"),
            &Capability::new("hardware_transcode", true, true)
        );
    }

    #[test]
    fn not_compiled_disables_ml_but_not_others() {
        let caps = select_capabilities(false, all_on());
        assert_eq!(
            find(&caps, "transcription"),
            &Capability::new("transcription", false, false)
        );
        assert_eq!(
            find(&caps, "translation"),
            &Capability::new("translation", false, false)
        );
        assert_eq!(
            find(&caps, "upscaling"),
            &Capability::new("upscaling", true, true)
        );
        assert_eq!(find(&caps, "tmdb"), &Capability::new("tmdb", true, true));
    }

    #[test]
    fn disabled_inputs_report_available_but_off() {
        let caps = select_capabilities(
            true,
            CapabilityInputs {
                transcription: false,
                translation: false,
                upscaling: false,
                opensubtitles: false,
                tmdb: false,
                tls: false,
                webhooks: false,
                content_fetch: false,
                hardware_transcode_available: false,
                hardware_transcode_enabled: false,
            },
        );
        assert_eq!(
            find(&caps, "content_fetch"),
            &Capability::new("content_fetch", true, false)
        );
        assert_eq!(
            find(&caps, "transcription"),
            &Capability::new("transcription", true, false)
        );
        assert_eq!(
            find(&caps, "upscaling"),
            &Capability::new("upscaling", true, false)
        );
        assert_eq!(find(&caps, "tmdb"), &Capability::new("tmdb", true, false));
        assert_eq!(
            find(&caps, "hardware_transcode"),
            &Capability::new("hardware_transcode", false, false)
        );
    }

    #[test]
    fn wrapper_uses_compiled_feature_flag() {
        let caps = server_capabilities(all_on());
        assert_eq!(
            find(&caps, "transcription").available,
            cfg!(feature = "enrichment")
        );
    }
}
