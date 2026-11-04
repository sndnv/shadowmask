function ThemeNames() as object
    return ["dark", "light", "retro"]
end function

function DefaultThemeName() as string
    return "retro"
end function

function ThemeTokens(name as string) as object
    themes = {
        dark: {
            bg: "0x0B1113FF",
            surface: "0x121A1DFF",
            surfaceAlt: "0x1A2529FF",
            text: "0xE6EEF0FF",
            muted: "0x90A3A8FF",
            border: "0x263338FF",
            accent: "0x35C9D6FF",
            accentContrast: "0x06171AFF",
            ok: "0x3FAE6FFF",
            warn: "0xD69A1EFF",
            danger: "0xE05A3CFF",
            dangerContrast: "0x1B0904FF",
            okBg: "0x13271FFF",
            warnBg: "0x2A2410FF",
            dangerBg: "0x241310FF",
            artBg: "0x06090BFF",
            isDark: true
        },
        light: {
            bg: "0xF5F7F8FF",
            surface: "0xFFFFFFFF",
            surfaceAlt: "0xEAF0F1FF",
            text: "0x12191CFF",
            muted: "0x566268FF",
            border: "0xD5DEE0FF",
            accent: "0x0E7D88FF",
            accentContrast: "0xFFFFFFFF",
            ok: "0x1D7A46FF",
            warn: "0x8F6000FF",
            danger: "0xC0392BFF",
            dangerContrast: "0xFFFFFFFF",
            okBg: "0xE4F3EAFF",
            warnBg: "0xFBF0DDFF",
            dangerBg: "0xFBE7E4FF",
            artBg: "0x0C1113FF",
            isDark: false
        },
        retro: {
            bg: "0x221913FF",
            surface: "0x30241AFF",
            surfaceAlt: "0x3D2F21FF",
            text: "0xF4E7D1FF",
            muted: "0xB9A285FF",
            border: "0x4D3C2AFF",
            accent: "0xE0913AFF",
            accentContrast: "0x241206FF",
            ok: "0x8FAA46FF",
            warn: "0xE8C749FF",
            danger: "0xEC7C5EFF",
            dangerContrast: "0x2A0F06FF",
            okBg: "0x29301AFF",
            warnBg: "0x37311AFF",
            dangerBg: "0x341C14FF",
            artBg: "0x140E08FF",
            isDark: true
        }
    }

    if not themes.DoesExist(name)
        return WithRowTints(themes[DefaultThemeName()])
    end if
    return WithRowTints(themes[name])
end function

function RowTintAlpha() as string
    return "1F"
end function

function WithRowTints(tokens as object) as object
    tokens.rowOk = WithAlpha(tokens.ok, RowTintAlpha())
    tokens.rowWarn = WithAlpha(tokens.warn, RowTintAlpha())
    tokens.rowDanger = WithAlpha(tokens.danger, RowTintAlpha())
    return tokens
end function

function WithAlpha(color as string, alpha as string) as string
    return Left(color, 8) + alpha
end function

function HardenedTintAlpha() as string
    return "4D"
end function

function TimelinePreviewAlpha() as string
    return "73"
end function

function HardenedTokens(tokens as object) as object
    raised = {}
    raised.Append(tokens)

    raised.muted = tokens.text
    raised.border = tokens.text
    raised.rowOk = WithAlpha(tokens.ok, HardenedTintAlpha())
    raised.rowWarn = WithAlpha(tokens.warn, HardenedTintAlpha())
    raised.rowDanger = WithAlpha(tokens.danger, HardenedTintAlpha())
    return raised
end function

function ThemedTokens(name as string, contrast as boolean) as object
    tokens = ThemeTokens(name)
    if not contrast then return tokens

    return HardenedTokens(tokens)
end function

function TickerSpeed() as float
    return 40.0 * DensityScale() / AuthoredScale()
end function

function TextureOpacity() as float
    return 0.06
end function

function BackdropOpacity() as float
    return 0.12
end function

function BackdropFadeSeconds() as float
    return 0.32
end function

function BackdropFadeInSeconds() as float
    return 0.6
end function

function BackdropFadeOutSeconds(immersive as boolean) as float
    if immersive then return 1.2
    return BackdropFadeSeconds()
end function

function FocusGrowPercent() as integer
    return 10
end function

function FocusGrowSeconds() as float
    return 0.2
end function

function LayoutSettleSeconds() as float
    return 0.1
end function

function SearchDebounceSeconds() as float
    return 0.35
end function

function SearchKeyboardWidth() as integer
    return 560
end function

function CanvasWidth() as integer
    return 1920
end function

function CanvasHeight() as integer
    return 1080
end function

function DensityScale() as float
    return 1.25
end function

function AuthoredScale() as float
    return 1.75
end function

function Px(value as float) as integer
    return Int(value * DensityScale() / AuthoredScale() + 0.5)
end function

function TypeScale() as object
    if m.DoesExist("smTypeScale") then return m.smTypeScale

    m.smTypeScale = {
        text2xl: Px(48),
        textXl: Px(36),
        textLg: Px(32),
        textBase: Px(28),
        textSm: Px(24),
        textXs: Px(20)
    }
    return m.smTypeScale
end function

function SpacingScale() as object
    if m.DoesExist("smSpacingScale") then return m.smSpacingScale

    m.smSpacingScale = {
        s1: Px(7),
        s2: Px(14),
        s3: Px(21),
        s4: Px(28),
        s5: Px(42),
        s6: Px(56),
        s8: Px(84),
        s10: Px(112)
    }
    return m.smSpacingScale
end function

function FontUri() as string
    return FontPathIn(ActiveFonts(), "regular")
end function

function BoldFontUri() as string
    return FontPathIn(ActiveFonts(), "medium")
end function

function BorderThickness() as integer
    return Px(3)
end function

function CornerRadius() as integer
    return 10
end function

function ActionControlHeight() as integer
    return Px(73)
end function

function ToggleControlHeight() as integer
    return Px(70)
end function

function GlyphControlHeight() as integer
    return Px(80)
end function

function GlyphIconSize() as integer
    return Px(40)
end function

function DialogWidth() as integer
    return Px(CanvasWidth() * 0.52)
end function

function DialogInnerWidth() as integer
    return DialogWidth() - SpacingScale().s5 * 2
end function

function DialogMaxHeight() as integer
    return Int(CanvasHeight() * 0.78)
end function

function DialogRowHeight() as integer
    return Px(76)
end function

function DialogMessageLines() as integer
    return 6
end function

function ScreenEdgeGap() as integer
    return 98
end function

function ScreenEndGap() as integer
    return 56
end function

function NavEdgeGap() as integer
    return 32
end function

function NavLeft(inset as boolean) as integer
    if not inset then return NavEdgeGap()
    return 112
end function

function NavTop(inset as boolean) as integer
    if not inset then return NavEdgeGap()
    return 76
end function

function NavPanelHeight(inset as boolean) as integer
    return CanvasHeight() - NavTop(inset) * 2
end function

function NavPanelColor(theme as object) as string
    return WithAlpha(theme.surface, "E6")
end function

function NavRowLit(focusPercent as float, listHasFocus as boolean) as boolean
    return listHasFocus and focusPercent > 0.5
end function

function NavRowLook(theme as object, lit as boolean) as object
    if lit then return { color: theme.accentContrast, bold: true }
    return { color: theme.muted, bold: false }
end function

function NavHintSize() as integer
    return SpacingScale().s8
end function

function NavHintChevronSize() as integer
    return SpacingScale().s4
end function

function NavHintLayout(open as boolean, railWidth as integer, inset as boolean) as object
    chevron = NavHintChevronSize()
    disc = NavHintSize()
    half = disc \ 2
    y = (CanvasHeight() - disc) \ 2
    centred = (disc - chevron) \ 2

    if open
        return {
            x: NavLeft(inset) + railWidth - half,
            y: y,
            disc: disc,
            clip: [half, 0, half, disc],
            opacity: 0.9,
            chevronX: half + (half - chevron) \ 2,
            chevronY: centred
        }
    end if

    return {
        x: NavLeft(inset),
        y: y,
        disc: disc,
        clip: [0, 0, disc, disc],
        opacity: 0.6,
        chevronX: centred,
        chevronY: centred
    }
end function

function ScrimColor(theme as object) as string
    return WithAlpha(theme.bg, "D9")
end function

function PausedMarkSize() as integer
    return Px(160)
end function

function PausedGlyphSize() as integer
    return Px(88)
end function

function LinkControlHeight() as integer
    return Int(TypeScale().textSm * 1.6)
end function

function LinkPadding() as integer
    return SpacingScale().s1
end function

function BorderColorFor(theme as object, focused as boolean) as string
    if focused then return theme.accent
    return theme.border
end function

function FocusRingInset() as integer
    return 3
end function

function FocusRingRadius() as integer
    return CornerRadius() + FocusRingInset()
end function

function ScrollBarWidth() as integer
    return Px(8)
end function

function ScrollThumbMinimum() as integer
    return Px(56)
end function

function ProgressTrackColor() as string
    return "0x00000073"
end function

function ProgressBarHeight() as integer
    return Px(8)
end function

function AccentTint(theme as object) as string
    return WithAlpha(theme.accent, "1F")
end function

function AverageGlyphRatio() as float
    return 0.62
end function

function DismissTint(theme as object) as string
    return WithAlpha(theme.accent, "59")
end function

function EstimatedTextWidth(text as dynamic, size as integer) as integer
    value = TextOrBlank(text)
    if Len(value) = 0 or size <= 0 then return 0
    return Int(Len(value) * size * AverageGlyphRatio())
end function

function MeasureLimit() as integer
    return 1000000
end function

function FontFamilyNames() as object
    return FamilyNamesFor(ActiveFonts())
end function

function FamilyNamesFor(fonts as dynamic) as object
    name = TextOrBlank(ValueAt(fonts, "name", ""))
    if IsBlank(name) then return { regular: "", bold: "" }
    return { regular: name, bold: name + " Medium" }
end function

sub LoadFontRegistry()
    if m.DoesExist("smFontFamilies") then return
    WatchFonts()

    m.smFontFamilies = {}
    m.smFonts = {}

    if FontRegistryRefused(m.global) then return
    registry = CreateObject("roFontRegistry")
    if registry = invalid
        NoteFontRegistryRefused(m.global)
        return
    end if

    for each uri in [FontUri(), BoldFontUri()]
        if not IsBlank(uri) then registry.Register(uri)
    end for

    families = registry.GetFamilies()
    if families <> invalid
        for each family in families
            name = TextOrBlank(family)
            if not IsBlank(name) then m.smFontFamilies[LCase(name)] = name
        end for
    end if

    m.smFontRegistry = registry
end sub

function FontRegistryRefused(shared as dynamic) as boolean
    if shared = invalid then return false
    if not shared.HasField("fontRegistryRefused") then return false
    return shared.fontRegistryRefused = true
end function

sub NoteFontRegistryRefused(shared as dynamic)
    if shared = invalid then return
    if shared.HasField("fontRegistryRefused") then shared.fontRegistryRefused = true
end sub

function FontFamilyFor(bold as boolean) as string
    names = FontFamilyNames()
    if IsBlank(names.regular) then return ""

    wanted = names.regular
    if bold then wanted = names.bold

    if m.smFontFamilies.DoesExist(LCase(wanted)) then return wanted
    if m.smFontFamilies.DoesExist(LCase(names.regular)) then return names.regular
    return ""
end function

function MeasuredFont(size as integer, bold as boolean) as dynamic
    LoadFontRegistry()
    if not m.DoesExist("smFontRegistry") then return invalid

    key = size.ToStr() + ":" + bold.ToStr()
    if m.smFonts.DoesExist(key) then return m.smFonts[key]

    font = invalid
    family = FontFamilyFor(bold)
    if IsBlank(FontUri())
        font = m.smFontRegistry.GetDefaultFont(size, bold, false)
    else if not IsBlank(family)
        synthetic = bold and family = FontFamilyNames().regular
        font = m.smFontRegistry.GetFont(family, size, synthetic, false)
    end if

    if type(font) <> "roFont" then font = invalid

    m.smFonts[key] = font
    return font
end function

function TextWidth(text as dynamic, size as integer, bold = false as boolean) as integer
    value = TextOrBlank(text)
    if Len(value) = 0 or size <= 0 then return 0

    font = MeasuredFont(size, bold)
    if font <> invalid
        measured = font.GetOneLineWidth(value, MeasureLimit())
        if measured > 0 then return Int(measured)
    end if

    measured = LabelTextWidth(value, size, bold)
    if measured > 0 then return measured

    return EstimatedTextWidth(value, size)
end function

function LabelTextWidth(value as string, size as integer, bold as boolean) as integer
    if not m.DoesExist("smMeasureLabels") then m.smMeasureLabels = {}

    key = size.ToStr() + ":" + bold.ToStr()
    label = m.smMeasureLabels[key]
    if label = invalid
        label = CreateObject("roSGNode", "Label")
        if label = invalid then return 0
        if bold then label.font = SizedBoldFont(size) else label.font = SizedFont(size)
        m.smMeasureLabels[key] = label
    end if

    label.text = value
    return Int(label.boundingRect().width)
end function

function ToastLabelWidth(measured as integer, slack as integer) as integer
    return measured + slack
end function

function SpaceWidth(size as integer) as integer
    width = TextWidth("n n", size) - TextWidth("nn", size)
    if width > 0 then return width
    return Int(size * 0.28)
end function

function TextLineSpacing() as integer
    return Px(8)
end function

function TextLinePitch(size as integer) as integer
    return FontLineHeight(size) + TextLineSpacing()
end function

function ChipHeight() as integer
    return Int(TypeScale().textSm * 1.8)
end function

function TextBlockHeight(size as integer, lines as integer) as integer
    if lines <= 0 then return 0
    return FontLineHeight(size) * lines + TextLineSpacing() * (lines - 1)
end function

function FontLineHeight(size as integer) as integer
    font = MeasuredFont(size, false)
    if font <> invalid
        height = font.GetOneLineHeight()
        if height > 0 then return Int(height)
    end if

    height = LabelLineHeight(size)
    if height > 0 then return height

    return Int(size * 1.35)
end function

function LabelLineHeight(size as integer) as integer
    if not m.DoesExist("smLineHeights") then m.smLineHeights = {}

    key = size.ToStr()
    if m.smLineHeights.DoesExist(key) then return m.smLineHeights[key]

    label = CreateObject("roSGNode", "Label")
    if label = invalid then return 0
    label.font = SizedFont(size)
    label.text = "Ag"
    height = Int(label.boundingRect().height)

    if height > 0 then m.smLineHeights[key] = height
    return height
end function

function SizedFont(size as float) as object
    return CachedFont("regular", size)
end function

function SizedBoldFont(size as float) as object
    return CachedFont("medium", size)
end function

function SizedItalicFont(size as float) as object
    return CachedFont("italic", size)
end function

function ItalicFontReady() as boolean
    return not IsBlank(FontPathIn(ActiveFonts(), "italic"))
end function

function CachedFont(role as string, size as float) as object
    if not m.DoesExist("smFontNodes") then m.smFontNodes = {}
    WatchFonts()

    key = role + ":" + Str(size).Trim()
    if m.smFontNodes.DoesExist(key) then return m.smFontNodes[key]

    font = FontNode(FontPathIn(ActiveFonts(), role), SystemFontName(role), size)
    m.smFontNodes[key] = font
    return font
end function

function FontNode(uri as string, systemName as string, size as float) as object
    if IsBlank(uri)
        holder = CreateObject("roSGNode", "Label")
        holder.font = systemName
        font = holder.font
    else
        font = CreateObject("roSGNode", "Font")
        font.uri = uri
    end if

    font.size = size
    return font
end function

function SystemFontName(role as string) as string
    if role = "medium" then return "font:MediumBoldSystemFont"
    return "font:MediumSystemFont"
end function

function ActiveFonts() as object
    shared = m.global
    if shared = invalid or not shared.HasField("fonts") then return {}
    if type(shared.fonts) <> "roAssociativeArray" then return {}
    return shared.fonts
end function

function FontPathIn(fonts as dynamic, role as string) as string
    return TextOrBlank(ValueAt(fonts, role, ""))
end function

sub WatchFonts()
    if m.DoesExist("smFontsWatched") then return
    m.smFontsWatched = true

    shared = m.global
    if shared <> invalid and shared.HasField("fonts") then shared.ObserveField("fonts", "onFontsChanged")
end sub

sub onFontsChanged()
    if m.DoesExist("smFontNodes")
        fonts = ActiveFonts()
        for each key in m.smFontNodes.Keys()
            uri = CachedFontUri(key, fonts)
            if IsBlank(uri) then m.smFontNodes.Delete(key) else m.smFontNodes[key].uri = uri
        end for
    end if

    ForgetTextMeasures()
end sub

function CachedFontUri(key as string, fonts as dynamic) as string
    return FontPathIn(fonts, Left(key, Instr(1, key, ":") - 1))
end function

sub ForgetTextMeasures()
    for each key in ["smFontFamilies", "smFonts", "smFontRegistry", "smMeasureLabels", "smLineHeights", "smMiddleEllipsis", "smWordMeasure", "smLineCount"]
        m.Delete(key)
    end for
end sub

function FontsDirectory() as string
    return "cachefs:/fonts"
end function

function FontRoles() as object
    return ["regular", "medium", "license"]
end function

function ItalicFontRoles() as object
    return ["italic", "italic_license"]
end function

function FontTimeoutSeconds() as integer
    return 300
end function

function FontJob(plan as object) as object
    return { directory: FontsDirectory(), files: plan.files, certificates: CertificatesFile(), timeout: FontTimeoutSeconds() }
end function

function FontCachePath(file as dynamic) as string
    hash = LCase(TextOrBlank(ValueAt(file, "sha256", "")))
    if not CreateObject("roRegex", "^[0-9a-f]{64}$", "").IsMatch(hash) then return ""

    url = TextOrBlank(ValueAt(file, "url", ""))
    extension = CreateObject("roRegex", "(\.[A-Za-z0-9]{1,5})$", "").Match(url)
    suffix = ""
    if extension.Count() > 1 then suffix = LCase(extension[1])

    return FontsDirectory() + "/" + hash + suffix
end function

function FontDownloads(fonts as dynamic, base as string) as dynamic
    if type(fonts) <> "roAssociativeArray" then return invalid

    name = TextOrBlank(ValueAt(fonts, "name", ""))
    if IsBlank(name) then return invalid

    files = FontRoleDownloads(fonts, FontRoles(), base)
    if files = invalid then return invalid

    italic = FontRoleDownloads(fonts, ItalicFontRoles(), base)
    if italic <> invalid then files.Append(italic)
    return { name: name, files: files }
end function

function FontRoleDownloads(fonts as object, roles as object, base as string) as dynamic
    files = []
    for each role in roles
        file = ValueAt(fonts, role, invalid)
        path = FontCachePath(file)
        url = TextOrBlank(ValueAt(file, "url", ""))
        size = ValueAt(file, "size", invalid)
        if IsBlank(path) or Left(url, 1) <> "/" or not IsNumber(size) then return invalid
        files.Push({ role: role, url: JoinUrl(base, url), path: path, size: size })
    end for
    return files
end function

function ActiveFontsFrom(plan as object) as object
    fonts = { name: plan.name }
    for each file in plan.files
        fonts[file.role] = file.path
    end for
    return fonts
end function

function DeviceResolutionName() as string
    return DeviceInfo().GetUIResolution().name
end function
