function RegistrySection() as string
    return "shadowmask"
end function

function ThemePrefKey() as string
    return "theme"
end function

function ValidThemeName(name as dynamic) as string
    wanted = LCase(TextOrBlank(name))
    for each known in ThemeNames()
        if known = wanted then return known
    end for
    return DefaultThemeName()
end function

function PrefSection() as object
    return CreateObject("roRegistrySection", RegistrySection())
end function

function ReadPref(key as string, fallback as string) as string
    section = PrefSection()

    value = ""
    if section.Exists(key) then value = section.Read(key)

    if IsBlank(value) then return fallback
    return value
end function

sub WritePref(key as string, value as string)
    section = PrefSection()
    section.Write(key, value)
    section.Flush()
end sub

function ReadThemeName() as string
    return ValidThemeName(ReadPref(ThemePrefKey(), DefaultThemeName()))
end function

sub WriteThemeName(name as string)
    WritePref(ThemePrefKey(), name)
end sub

function ReadFlag(key as string, fallback as boolean) as boolean
    stored = ReadPref(key, "")
    if IsBlank(stored) then return fallback

    return stored = "1"
end function

sub WriteFlag(key as string, value as boolean)
    if value
        WritePref(key, "1")
        return
    end if
    WritePref(key, "0")
end sub

function AutoplayNextKey() as string
    return "autoplayNext"
end function

function DiagnosticsKey() as string
    return "diagnostics"
end function

function ReadAutoplayNext() as boolean
    return ReadFlag(AutoplayNextKey(), true)
end function

sub WriteAutoplayNext(value as boolean)
    WriteFlag(AutoplayNextKey(), value)
end sub

function ReadDiagnostics() as boolean
    return ReadFlag(DiagnosticsKey(), false)
end function

sub WriteDiagnostics(value as boolean)
    WriteFlag(DiagnosticsKey(), value)
end sub

function RemainingTimeKey() as string
    return "remainingTime"
end function

function ReadRemainingTime() as boolean
    return ReadFlag(RemainingTimeKey(), false)
end function

sub WriteRemainingTime(value as boolean)
    WriteFlag(RemainingTimeKey(), value)
end sub

function AutoplayDelayKey() as string
    return "autoplayDelay"
end function

function ReadAutoplayDelay() as integer
    stored = ReadPref(AutoplayDelayKey(), "")
    if IsBlank(stored) then return DefaultCountdownSeconds()

    return ClampedCountdownSeconds(Int(Val(stored)))
end function

sub WriteAutoplayDelay(seconds as integer)
    WritePref(AutoplayDelayKey(), NumberText(ClampedCountdownSeconds(seconds)))
end sub

function HighContrastKey() as string
    return "highContrast"
end function

function ContrastRequest() as string
    return "contrast"
end function

function ReadHighContrast() as boolean
    return ReadFlag(HighContrastKey(), false)
end function

sub WriteHighContrast(value as boolean)
    WriteFlag(HighContrastKey(), value)
end sub

function OverrideKey(name as string) as string
    return "caps." + name
end function

function ReadCapabilityOverrides() as object
    overrides = CapabilityOverrides()

    overrides.maxHeight = Int(Val(ReadPref(OverrideKey("maxHeight"), "0")))
    overrides.maxFrameRate = Int(Val(ReadPref(OverrideKey("maxFrameRate"), "0")))
    overrides.hdr = ValidOption(OverrideHdrOptions(), ReadPref(OverrideKey("hdr"), AutoOverride()), AutoOverride())

    for each codec in KnownVideoCodecs()
        stored = ValidOption(OverrideCodecOptions(), ReadPref(OverrideKey(codec), AutoOverride()), AutoOverride())
        if stored <> AutoOverride() then overrides.codecs[codec] = stored
    end for

    return overrides
end function

sub WriteCapabilityOverrides(overrides as object)
    section = PrefSection()
    section.Write(OverrideKey("maxHeight"), Int(ValueAt(overrides, "maxHeight", 0)).ToStr())
    section.Write(OverrideKey("maxFrameRate"), Int(ValueAt(overrides, "maxFrameRate", 0)).ToStr())
    section.Write(OverrideKey("hdr"), TextOrBlank(ValueAt(overrides, "hdr", AutoOverride())))

    codecs = ValueAt(overrides, "codecs", {})
    for each codec in KnownVideoCodecs()
        section.Write(OverrideKey(codec), TextOrBlank(ValueAt(codecs, codec, AutoOverride())))
    end for

    section.Flush()
end sub

function ListSortKey(kind as string) as string
    return "listSort." + kind
end function

function ListOrderKey(kind as string) as string
    return "listOrder." + kind
end function

function ReadListQuery(kind as string) as object
    query = DefaultListQuery()
    query.sort = ValidOption(SortOptions(), ReadPref(ListSortKey(kind), DefaultSort()), DefaultSort())
    query.order = ValidOption(OrderOptions(), ReadPref(ListOrderKey(kind), DefaultOrder()), DefaultOrder())
    return query
end function

sub WriteListQuery(kind as string, query as object)
    section = PrefSection()
    section.Write(ListSortKey(kind), TextOrBlank(ValueAt(query, "sort", DefaultSort())))
    section.Write(ListOrderKey(kind), TextOrBlank(ValueAt(query, "order", DefaultOrder())))
    section.Flush()
end sub
