function BundledFontName() as string
    return "Roboto"
end function

function BundledFontLicense() as string
    return "SIL Open Font License 1.1"
end function

function BundledLicensePath() as string
    return "pkg:/fonts/Roboto-OFL.txt"
end function

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

function AttributionRows() as object
    return [
        { label: BundledFontName(), values: [BundledFontLicense()] }
    ]
end function

function BundledLicenseText(path = "" as string) as string
    source = path
    if IsBlank(source) then source = BundledLicensePath()

    return TextOrBlank(ReadAsciiFile(source))
end function
