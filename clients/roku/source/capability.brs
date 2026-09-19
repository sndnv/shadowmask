function VideoProbeSpecs() as object
    return [
        { codec: "h264", profile: "main", level: "4.0", bitDepth: 8 },
        { codec: "h264", profile: "main", level: "4.1", bitDepth: 8 },
        { codec: "h264", profile: "main", level: "4.2", bitDepth: 8 },
        { codec: "h264", profile: "high", level: "4.0", bitDepth: 8 },
        { codec: "h264", profile: "high", level: "4.1", bitDepth: 8 },
        { codec: "h264", profile: "high", level: "4.2", bitDepth: 8 },
        { codec: "hevc", profile: "main", level: "5.0", bitDepth: 8 },
        { codec: "hevc", profile: "main", level: "5.1", bitDepth: 8 },
        { codec: "hevc", profile: "main10", level: "5.0", bitDepth: 10 },
        { codec: "hevc", profile: "main10", level: "5.1", bitDepth: 10 }
    ]
end function

function AudioProbeSpecs() as object
    return ["aac", "ac3", "eac3"]
end function

function UnplayableCodecs() as object
    return ["av1"]
end function

function ReportableCodec(codec as string) as boolean
    for each name in UnplayableCodecs()
        if name = codec then return false
    end for
    return true
end function

function LevelRank(level as dynamic) as float
    text = TextOrBlank(level)
    if IsBlank(text) then return 0.0
    return Val(text)
end function

function VideoCaps(answers as dynamic) as object
    caps = []
    if type(answers) <> "roArray" then return caps

    order = []
    found = {}
    for each reply in answers
        codec = LCase(TextOrBlank(ValueAt(reply, "codec", "")))
        if ValueAt(reply, "ok", false) = true and not IsBlank(codec) and ReportableCodec(codec)
            level = TextOrBlank(ValueAt(reply, "level", ""))
            depth = Int(ValueAt(reply, "bitDepth", 8))

            if not found.DoesExist(codec)
                order.Push(codec)
                found[codec] = { codec: codec, max_level: level, max_bit_depth: depth }
            else
                entry = found[codec]
                if LevelRank(level) > LevelRank(entry.max_level) then entry.max_level = level
                if depth > entry.max_bit_depth then entry.max_bit_depth = depth
            end if
        end if
    end for

    for each codec in order
        entry = found[codec]
        if IsBlank(entry.max_level) then entry.Delete("max_level")
        caps.Push(entry)
    end for
    return caps
end function

function AudioCaps(answers as dynamic, channels as integer) as object
    caps = []
    if type(answers) <> "roArray" then return caps

    reported = channels
    if reported <= 0 then reported = 2

    seen = {}
    for each reply in answers
        codec = LCase(TextOrBlank(ValueAt(reply, "codec", "")))
        if ValueAt(reply, "ok", false) = true and not IsBlank(codec) and not seen.DoesExist(codec)
            seen[codec] = true
            caps.Push({ codec: codec, max_channels: reported })
        end if
    end for
    return caps
end function

function OutputChannels(name as dynamic) as integer
    text = LCase(TextOrBlank(name))
    if Instr(1, text, "7.1") > 0 then return 8
    if Instr(1, text, "5.1") > 0 then return 6
    return 2
end function

function LeadingDigits(text as string, from as integer) as string
    digits = ""
    index = from
    while index <= Len(text)
        character = Mid(text, index, 1)
        if character < "0" or character > "9" then exit while
        digits = digits + character
        index = index + 1
    end while
    return digits
end function

function FrameRateFromMode(mode as dynamic) as integer
    text = LCase(TextOrBlank(mode))
    marker = Instr(1, text, "p")
    if marker <= 0 then return 0

    digits = LeadingDigits(text, marker + 1)
    if Len(digits) = 0 then return 0
    return Int(Val(digits))
end function

function PictureSizes() as object
    return {
        "480": [720, 480],
        "576": [720, 576],
        "720": [1280, 720],
        "1080": [1920, 1080],
        "2160": [3840, 2160],
        "4320": [7680, 4320]
    }
end function

function PictureFromMode(mode as dynamic, display as dynamic) as dynamic
    sizes = PictureSizes()
    vertical = LeadingDigits(LCase(TextOrBlank(mode)), 1)
    if sizes.DoesExist(vertical) then return { width: sizes[vertical][0], height: sizes[vertical][1] }

    width = Int(ValueAt(display, "w", 0))
    height = Int(ValueAt(display, "h", 0))
    if width > 0 and height > 0 then return { width: width, height: height }

    return invalid
end function

function HdrNames(properties as dynamic) as object
    names = []
    if ValueAt(properties, "Hdr10", false) = true then names.Push("hdr10")
    if ValueAt(properties, "HLG", false) = true then names.Push("hlg")
    return names
end function

function ReportContainers(mkv as boolean) as object
    if mkv then return ["mp4", "mkv", "ts", "hls"]
    return ["mp4", "ts", "hls"]
end function

function DecodingReport(measurements as dynamic) as object
    report = {}
    if type(measurements) <> "roAssociativeArray" then return report

    video = VideoCaps(ValueAt(measurements, "video", invalid))
    if video.Count() > 0 then report.video = video

    audio = AudioCaps(ValueAt(measurements, "audio", invalid), Int(ValueAt(measurements, "channels", 0)))
    if audio.Count() > 0 then report.audio = audio

    properties = ValueAt(measurements, "hdr", invalid)
    if type(properties) = "roAssociativeArray" then report.hdr = HdrNames(properties)

    report.containers = ReportContainers(ValueAt(measurements, "mkv", false) = true)

    mode = ValueAt(measurements, "mode", "")
    picture = PictureFromMode(mode, ValueAt(measurements, "display", invalid))
    if picture <> invalid
        report.max_width = picture.width
        report.max_height = picture.height
    end if

    rate = FrameRateFromMode(mode)
    if rate > 0 then report.max_frame_rate = rate

    return report
end function

function KnownVideoCodecs() as object
    return ["h264", "hevc", "vp9", "av1"]
end function

function HdrFormats() as object
    return ["hdr10", "hdr10plus", "hlg"]
end function

function PictureHeights() as object
    return [2160, 1440, 1080, 720]
end function

function FrameRateCaps() as object
    return [60, 30]
end function

function AddedCodecBitDepth() as integer
    return 10
end function

function AutoOverride() as string
    return "auto"
end function

function CapabilityOverrides() as object
    return { codecs: {}, maxHeight: 0, maxFrameRate: 0, hdr: AutoOverride() }
end function

function OverrideCodecOptions() as object
    return [
        { value: AutoOverride(), label: Phrase("caps.detected") },
        { value: "hardware", label: Phrase("caps.hardware") },
        { value: "software", label: Phrase("caps.software") },
        { value: "unsupported", label: Phrase("caps.unsupported") }
    ]
end function

function OverrideHdrOptions() as object
    return [
        { value: AutoOverride(), label: Phrase("caps.detected") },
        { value: "allow", label: Phrase("caps.allow") },
        { value: "deny", label: Phrase("caps.deny") }
    ]
end function

function PictureHeightOptions() as object
    options = [{ value: "0", label: Phrase("caps.detected") }]
    for each height in PictureHeights()
        options.Push({ value: height.ToStr(), label: PhraseWith("caps.upTo", { height: height }) })
    end for
    return options
end function

function FrameRateOptions() as object
    options = [{ value: "0", label: Phrase("caps.detected") }]
    for each rate in FrameRateCaps()
        options.Push({ value: rate.ToStr(), label: PhraseWith("caps.upToRate", { rate: rate }) })
    end for
    return options
end function

function OverrideFor(overrides as dynamic, codec as string) as string
    stored = ValueAt(ValueAt(overrides, "codecs", {}), codec, AutoOverride())

    return TextOrBlank(stored)
end function

function OverridesChanged(overrides as dynamic) as boolean
    if Int(ValueAt(overrides, "maxHeight", 0)) > 0 then return true
    if Int(ValueAt(overrides, "maxFrameRate", 0)) > 0 then return true
    if TextOrBlank(ValueAt(overrides, "hdr", AutoOverride())) <> AutoOverride() then return true

    codecs = ValueAt(overrides, "codecs", {})
    if type(codecs) <> "roAssociativeArray" then return false

    for each codec in codecs
        if TextOrBlank(codecs[codec]) <> AutoOverride() then return true
    end for
    return false
end function

function KnownVideoCodec(codec as dynamic) as boolean
    wanted = LCase(TextOrBlank(codec))
    for each known in KnownVideoCodecs()
        if known = wanted then return true
    end for
    return false
end function

function FindVideoCap(caps as dynamic, codec as string) as dynamic
    if type(caps) <> "roArray" then return invalid

    for each cap in caps
        if LCase(TextOrBlank(ValueAt(cap, "codec", ""))) = codec then return cap
    end for
    return invalid
end function

function ForcedVideoCap(codec as string, found as dynamic, smooth as boolean) as object
    cap = { codec: codec, max_bit_depth: AddedCodecBitDepth(), smooth: smooth }
    if found = invalid then return cap

    cap.max_bit_depth = Int(ValueAt(found, "max_bit_depth", AddedCodecBitDepth()))
    level = TextOrBlank(ValueAt(found, "max_level", ""))
    if not IsBlank(level) then cap.max_level = level

    return cap
end function

function OverriddenVideo(measured as dynamic, overrides as dynamic) as object
    caps = ValueAt(measured, "video", invalid)
    built = []

    for each codec in KnownVideoCodecs()
        found = FindVideoCap(caps, codec)
        choice = OverrideFor(overrides, codec)

        if choice = "hardware"
            built.Push(ForcedVideoCap(codec, found, true))
        else if choice = "software"
            built.Push(ForcedVideoCap(codec, found, false))
        else if choice <> "unsupported" and found <> invalid
            built.Push(found)
        end if
    end for

    if type(caps) = "roArray"
        for each cap in caps
            if not KnownVideoCodec(ValueAt(cap, "codec", "")) then built.Push(cap)
        end for
    end if

    if built.Count() = 0 and type(caps) = "roArray" then return caps
    return built
end function

function OverriddenHdr(measured as dynamic, overrides as dynamic) as dynamic
    choice = TextOrBlank(ValueAt(overrides, "hdr", AutoOverride()))
    found = ValueAt(measured, "hdr", invalid)

    if choice = "deny" then return []
    if choice <> "allow" then return found
    if type(found) = "roArray" and found.Count() > 0 then return found

    return HdrFormats()
end function

function ApplyOverrides(measured as dynamic, overrides as dynamic) as object
    report = {}
    if type(measured) = "roAssociativeArray" then report.Append(measured)
    if not OverridesChanged(overrides) then return report

    video = OverriddenVideo(measured, overrides)
    if video.Count() > 0
        report.video = video
    else
        report.Delete("video")
    end if

    hdr = OverriddenHdr(measured, overrides)
    if hdr = invalid
        report.Delete("hdr")
    else
        report.hdr = hdr
    end if

    height = Int(ValueAt(overrides, "maxHeight", 0))
    if height > 0 then report.max_height = height

    rate = Int(ValueAt(overrides, "maxFrameRate", 0))
    if rate > 0 then report.max_frame_rate = rate

    return report
end function

function CapabilitiesBody(decoding as dynamic, profileVersion as integer) as object
    version = profileVersion
    if version <= 0 then version = ExpectedProfileVersion()

    body = { platform: ClientPlatform(), profile_version: version }
    if type(decoding) = "roAssociativeArray" and decoding.Count() > 0 then body.decoding = decoding
    return body
end function

function PictureValues(decoding as dynamic) as object
    width = Int(ValueAt(decoding, "max_width", 0))
    height = Int(ValueAt(decoding, "max_height", 0))
    if width <= 0 or height <= 0 then return [Phrase("status.unavailable")]

    size = width.ToStr() + "×" + height.ToStr()
    return [JoinParts([size, FormatFrameRate(ValueAt(decoding, "max_frame_rate", invalid))])]
end function

function VideoValues(decoding as dynamic) as object
    lines = []
    caps = ValueAt(decoding, "video", invalid)
    if type(caps) = "roArray"
        for each cap in caps
            codec = UCase(TextOrBlank(ValueAt(cap, "codec", "")))
            depth = PhraseWith("track.bitDepth", { bits: Int(ValueAt(cap, "max_bit_depth", 8)) })
            lines.Push(JoinParts([codec, TextOrBlank(ValueAt(cap, "max_level", "")), depth]))
        end for
    end if

    if lines.Count() = 0 then return [Phrase("diagnostics.notMeasured")]
    return lines
end function

function AudioValues(decoding as dynamic) as object
    lines = []
    caps = ValueAt(decoding, "audio", invalid)
    if type(caps) = "roArray"
        for each cap in caps
            codec = UCase(TextOrBlank(ValueAt(cap, "codec", "")))
            lines.Push(JoinParts([codec, ChannelLabel(Int(ValueAt(cap, "max_channels", 0)))]))
        end for
    end if

    if lines.Count() = 0 then return [Phrase("diagnostics.notMeasured")]
    return lines
end function

function HdrValues(decoding as dynamic) as object
    names = ValueAt(decoding, "hdr", invalid)
    if type(names) <> "roArray" then return [Phrase("diagnostics.notMeasured")]

    labels = []
    for each name in names
        label = HdrLabel(name)
        if not IsBlank(label) then labels.Push(label)
    end for

    if labels.Count() = 0 then return [Phrase("diagnostics.none")]
    return [JoinParts(labels)]
end function

function ContainerValues(decoding as dynamic) as object
    names = ValueAt(decoding, "containers", invalid)
    if type(names) <> "roArray" or names.Count() = 0 then return [Phrase("diagnostics.notMeasured")]

    labels = []
    for each name in names
        labels.Push(UCase(TextOrBlank(name)))
    end for
    return [JoinParts(labels)]
end function

function ProfileVersionValues(profileVersion as integer) as object
    if profileVersion <= 0 then return [Phrase("status.unavailable")]
    return [profileVersion.ToStr()]
end function

function PlaybackValues(negotiated as dynamic) as object
    mode = TextOrBlank(ValueAt(negotiated, "mode", ""))
    if IsBlank(mode) then return [Phrase("diagnostics.noPlaybackYet")]

    return [JoinParts([mode, TextOrBlank(ValueAt(negotiated, "container", ""))])]
end function

function DiagnosticColumns() as integer
    return 4
end function

function DiagnosticRows(decoding as dynamic, profileVersion as integer, negotiated as dynamic) as object
    return [
        { label: Phrase("diagnostics.picture"), values: PictureValues(decoding) },
        { label: Phrase("heading.video"), values: VideoValues(decoding) },
        { label: Phrase("heading.audio"), values: AudioValues(decoding) },
        { label: Phrase("diagnostics.hdr"), values: HdrValues(decoding) },
        { label: Phrase("diagnostics.containers"), values: ContainerValues(decoding) },
        { label: Phrase("diagnostics.profileVersion"), values: ProfileVersionValues(profileVersion) },
        { label: Phrase("diagnostics.lastPlayback"), values: PlaybackValues(negotiated) }
    ]
end function

function ProbeAnswered(reply as dynamic) as boolean
    return ValueAt(reply, "result", false) = true
end function

function ProbeVideo(device as object) as object
    answers = []
    if device.CanDecodeVideo = invalid then return answers

    for each spec in VideoProbeSpecs()
        reply = device.CanDecodeVideo({ Codec: spec.codec, Profile: spec.profile, Level: spec.level })
        answers.Push({ codec: spec.codec, level: spec.level, bitDepth: spec.bitDepth, ok: ProbeAnswered(reply) })
    end for
    return answers
end function

function ProbeAudio(device as object) as object
    answers = []
    if device.CanDecodeAudio = invalid then return answers

    for each codec in AudioProbeSpecs()
        answers.Push({ codec: codec, ok: ProbeAnswered(device.CanDecodeAudio({ Codec: codec })) })
    end for
    return answers
end function

function ProbeDisplay(device as object) as dynamic
    if device.GetDisplayProperties = invalid then return invalid

    properties = device.GetDisplayProperties()
    if type(properties) <> "roAssociativeArray" then return invalid
    return properties
end function

function ProbeVideoMode(device as object) as string
    if device.GetVideoMode = invalid then return ""
    return TextOrBlank(device.GetVideoMode())
end function

function ProbeDisplaySize(device as object) as dynamic
    if device.GetDisplaySize = invalid then return invalid
    return device.GetDisplaySize()
end function

function ProbeAudioChannels(device as object) as integer
    if device.GetAudioOutputChannel = invalid then return 0
    return OutputChannels(device.GetAudioOutputChannel())
end function

function MkvDeclared() as boolean
    info = CreateObject("roAppInfo")
    if info = invalid then return false
    return info.GetValue("requires_mkv") = "1"
end function

function DeviceInfo() as dynamic
    if m.smDevice = invalid then m.smDevice = CreateObject("roDeviceInfo")
    return m.smDevice
end function

function MeasureDecoding() as object
    device = DeviceInfo()
    if device = invalid then return {}

    return DecodingReport({
        video: ProbeVideo(device),
        audio: ProbeAudio(device),
        channels: ProbeAudioChannels(device),
        hdr: ProbeDisplay(device),
        mode: ProbeVideoMode(device),
        display: ProbeDisplaySize(device),
        mkv: MkvDeclared()
    })
end function

function MeasuredDecoding() as object
    stored = m.global.decoding
    if type(stored) = "roAssociativeArray" and stored.Count() > 0 then return stored

    measured = MeasureDecoding()
    m.global.decoding = measured
    return measured
end function

function DeviceDecoding() as object
    return ApplyOverrides(MeasuredDecoding(), ReadCapabilityOverrides())
end function
