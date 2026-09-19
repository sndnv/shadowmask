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
    return 40.0
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

function TypeScale() as object
    if m.DoesExist("smTypeScale") then return m.smTypeScale

    m.smTypeScale = {
        text2xl: 48,
        textXl: 36,
        textLg: 32,
        textBase: 28,
        textSm: 24,
        textXs: 20
    }
    return m.smTypeScale
end function

function SpacingScale() as object
    if m.DoesExist("smSpacingScale") then return m.smSpacingScale

    m.smSpacingScale = {
        s1: 7,
        s2: 14,
        s3: 21,
        s4: 28,
        s5: 42,
        s6: 56,
        s8: 84,
        s10: 112
    }
    return m.smSpacingScale
end function

function FontUri() as string
    return "pkg:/fonts/Roboto-Regular.ttf"
end function

function BoldFontUri() as string
    return "pkg:/fonts/Roboto-Medium.ttf"
end function

function BorderThickness() as integer
    return 3
end function

function CornerRadius() as integer
    return 10
end function

function ActionControlHeight() as integer
    return 73
end function

function ToggleControlHeight() as integer
    return 70
end function

function GlyphControlHeight() as integer
    return 80
end function

function GlyphIconSize() as integer
    return 40
end function

function DialogWidth() as integer
    return Int(CanvasWidth() * 0.52)
end function

function DialogMaxHeight() as integer
    return Int(CanvasHeight() * 0.78)
end function

function DialogRowHeight() as integer
    return 76
end function

function DialogMessageLines() as integer
    return 6
end function

function ScrimColor(theme as object) as string
    return WithAlpha(theme.bg, "D9")
end function

function PausedMarkSize() as integer
    return 160
end function

function PausedGlyphSize() as integer
    return 88
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
    return 4
end function

function ScrollBarWidth() as integer
    return 8
end function

function ScrollThumbMinimum() as integer
    return 56
end function

function ProgressTrackColor() as string
    return "0x00000073"
end function

function ProgressBarHeight() as integer
    return 8
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
    return { regular: "Roboto", bold: "Roboto Medium" }
end function

sub LoadFontRegistry()
    if m.DoesExist("smFontFamilies") then return

    m.smFontFamilies = {}
    m.smFonts = {}

    registry = CreateObject("roFontRegistry")
    if registry = invalid then return

    registry.Register(FontUri())
    registry.Register(BoldFontUri())

    families = registry.GetFamilies()
    if families <> invalid
        for each family in families
            name = TextOrBlank(family)
            if not IsBlank(name) then m.smFontFamilies[LCase(name)] = name
        end for
    end if

    m.smFontRegistry = registry
end sub

function FontFamilyFor(bold as boolean) as string
    names = FontFamilyNames()

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
    if not IsBlank(family)
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

    return EstimatedTextWidth(value, size)
end function

function SpaceWidth(size as integer) as integer
    width = TextWidth("n n", size) - TextWidth("nn", size)
    if width > 0 then return width
    return Int(size * 0.28)
end function

function TextLineSpacing() as integer
    return 8
end function

function TextLinePitch(size as integer) as integer
    return FontLineHeight(size) + TextLineSpacing()
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

    return Int(size * 1.35)
end function

function SizedFont(size as float) as object
    return CachedFont(FontUri(), size)
end function

function SizedBoldFont(size as float) as object
    return CachedFont(BoldFontUri(), size)
end function

function CachedFont(uri as string, size as float) as object
    if not m.DoesExist("smFontNodes") then m.smFontNodes = {}

    key = uri + ":" + Str(size).Trim()
    if m.smFontNodes.DoesExist(key) then return m.smFontNodes[key]

    font = CreateObject("roSGNode", "Font")
    font.uri = uri
    font.size = size

    m.smFontNodes[key] = font
    return font
end function

function DeviceResolutionName() as string
    return DeviceInfo().GetUIResolution().name
end function
