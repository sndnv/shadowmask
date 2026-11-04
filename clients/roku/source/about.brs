function AppVersion() as string
    info = CreateObject("roAppInfo")
    if info = invalid then return ""

    parts = []
    for each key in ["major_version", "minor_version", "build_version"]
        value = TextOrBlank(info.GetValue(key))
        if IsBlank(value) then return ""
        parts.Push(value)
    end for

    return parts.Join(".")
end function

function AboutTitle() as string
    name = Phrase("app.name")
    version = AppVersion()
    if IsBlank(version) then return name

    return name + " " + version
end function

function ChannelId() as string
    info = CreateObject("roAppInfo")
    if info = invalid then return ""
    return TextOrBlank(info.GetID())
end function

function AttributionRows(fonts = invalid as dynamic) as object
    name = FontSetName(fonts)
    if IsBlank(name) then return [{ label: Phrase("about.systemFont"), values: [Phrase("about.systemFontSource")] }]

    return [{ label: name, values: [Phrase("about.fontSource")] }]
end function

function FontLine(fonts = invalid as dynamic) as string
    if IsBlank(FontSetName(fonts)) then return Phrase("about.systemFontLine")
    return Phrase("about.font")
end function

function FontSetName(fonts = invalid as dynamic) as string
    source = fonts
    if source = invalid then source = ActiveFonts()
    return TextOrBlank(ValueAt(source, "name", ""))
end function

function FontLicenseText(path as string) as string
    if IsBlank(path) then return ""
    return TextOrBlank(ReadAsciiFile(path))
end function

function FontLicensesText(fonts = invalid as dynamic) as string
    source = fonts
    if source = invalid then source = ActiveFonts()

    texts = []
    for each role in ["license", "italic_license"]
        text = FontLicenseText(FontPathIn(source, role))
        if not IsBlank(text) then texts.Push(text)
    end for
    return texts.Join(Chr(10) + Chr(10))
end function
