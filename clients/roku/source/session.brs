function ServerUrlKey() as string
    return "serverUrl"
end function

function TokenKey() as string
    return "token"
end function

function DeviceNameKey() as string
    return "deviceName"
end function

function DeviceIdKey() as string
    return "deviceId"
end function

function CredentialKeys() as object
    return [TokenKey(), DeviceIdKey()]
end function

function SessionKeys() as object
    return [ServerUrlKey(), DeviceNameKey(), TokenKey(), DeviceIdKey()]
end function

function IsPairedWith(serverUrl as dynamic, token as dynamic) as boolean
    return not IsBlank(serverUrl) and not IsBlank(token)
end function

function ReadServerUrl() as string
    return ReadPref(ServerUrlKey(), "")
end function

sub WriteServerUrl(url as string)
    WritePref(ServerUrlKey(), url)
end sub

function ReadToken() as string
    return ReadPref(TokenKey(), "")
end function

sub WriteToken(token as string)
    WritePref(TokenKey(), token)
end sub

function ReadDeviceName() as string
    stored = ReadPref(DeviceNameKey(), "")
    if not IsBlank(stored) then return stored
    return DefaultDeviceName()
end function

sub WriteDeviceName(name as string)
    WritePref(DeviceNameKey(), name)
end sub

function ReadDeviceId() as string
    return ReadPref(DeviceIdKey(), "")
end function

sub WriteDeviceId(id as string)
    WritePref(DeviceIdKey(), id)
end sub

function IsPaired() as boolean
    return IsPairedWith(ReadServerUrl(), ReadToken())
end function

function DefaultDeviceName() as string
    device = DeviceInfo()

    friendly = device.GetFriendlyName()
    if not IsBlank(friendly) then return friendly

    model = device.GetModelDisplayName()
    if not IsBlank(model) then return model

    return "Roku"
end function

function NotSetValues() as object
    return [Phrase("status.notSet")]
end function

function LanguageValues(codes as dynamic) as object
    if type(codes) <> "roArray" or codes.Count() = 0 then return NotSetValues()

    names = []
    for each code in codes
        names.Push(UCase(TextOrBlank(code)))
    end for
    return [JoinParts(names)]
end function

function NumberValues(value as dynamic) as object
    text = NumberText(value)
    if IsBlank(text) then return NotSetValues()

    return [text]
end function

function RatingValues(rating as dynamic) as object
    label = ContentRatingLabel(rating)
    if IsBlank(label) then return NotSetValues()

    return [label]
end function

function ProfileRows(user as dynamic) as object
    username = TextOrBlank(ValueAt(user, "username", ""))
    if IsBlank(username) then username = Phrase("state.loading")

    return [
        { label: Phrase("fact.username"), values: [username] },
        { label: Phrase("fact.role"), values: [TextOrBlank(ValueAt(user, "role", ""))] },
        { label: Phrase("fact.preferredAudio"), values: LanguageValues(ValueAt(user, "preferred_audio", invalid)) },
        { label: Phrase("fact.preferredSubtitle"), values: LanguageValues(ValueAt(user, "preferred_subtitle", invalid)) },
        { label: Phrase("fact.maximumRating"), values: RatingValues(ValueAt(user, "max_content_rating", invalid)) },
        { label: Phrase("fact.concurrentStreams"), values: NumberValues(ValueAt(user, "concurrent_stream_limit", invalid)) },
        { label: Phrase("fact.bitrateCap"), values: NumberValues(ValueAt(user, "bitrate_cap", invalid)) }
    ]
end function

sub ClearKeys(keys as object)
    section = PrefSection()
    for each key in keys
        if section.Exists(key) then section.Delete(key)
    end for
    section.Flush()
end sub

sub ClearToken()
    ClearKeys(CredentialKeys())
end sub

sub ClearSession()
    ClearKeys(SessionKeys())
end sub
