function CardImageWidthFor(cardWidth as dynamic) as integer
    width = RailCardWidth()
    if cardWidth <> invalid and Int(cardWidth) > 0 then width = Int(cardWidth)

    return Int((width - BorderThickness() * 2) * ArtSizeTolerance())
end function

function ArtSizeTolerance() as float
    return 0.95
end function

function PortraitBackdropScale() as float
    return 8 / 3
end function

function BackdropImageWidth() as integer
    return 1920
end function

function PosterPlaceholder() as string
    return "pkg:/images/art-movie.png"
end function

function LandscapePlaceholder() as string
    return "pkg:/images/art-landscape.png"
end function

function PersonPlaceholder() as string
    return "pkg:/images/art-person.png"
end function

function PlaylistPlaceholder() as string
    return "pkg:/images/art-playlist.png"
end function

function PlaceholderGlyphSize() as integer
    return Px(96)
end function

function GlyphUri(name as string) as string
    if IsBlank(name) then return ""
    return "pkg:/images/" + name + ".png"
end function

function ChevronUri(direction as string) as string
    return GlyphUri("chevron-" + direction)
end function

function ChevronSize() as integer
    return Px(44)
end function

function IsCut(theme as dynamic) as boolean
    return ValueAt(theme, "cut", false) = true
end function

function ShapeUri(theme as dynamic, cut as string, rounded as string) as string
    if IsCut(theme) then return GlyphUri(cut)
    return GlyphUri(rounded)
end function

function BoxFillUri(theme as dynamic) as string
    return ShapeUri(theme, "cut-fill.9", "button-fill.9")
end function

function BoxLineUri(theme as dynamic) as string
    return ShapeUri(theme, "cut-line.9", "button-line.9")
end function

function FocusRingUri(theme as dynamic) as string
    return ShapeUri(theme, "cut-focus.9", "focus-ring.9")
end function

function ChipCapUri(side as string, theme as dynamic) as string
    return ShapeUri(theme, "chip-cut-" + side, "chip-cap-" + side)
end function

function ChipCapLineUri(side as string, theme as dynamic) as string
    return ShapeUri(theme, "chip-cut-" + side + "-line", "chip-cap-" + side + "-line")
end function

function DiscUri(theme as dynamic) as string
    return ShapeUri(theme, "hex-disc", "disc")
end function

function RingUri(theme as dynamic) as string
    return ShapeUri(theme, "hex-ring", "ring")
end function

function WatchedDiscUri(theme as dynamic) as string
    return ShapeUri(theme, "watched-hex-disc", "watched-disc")
end function

function WatchedRingUri(theme as dynamic) as string
    return ShapeUri(theme, "watched-hex-ring", "watched-ring")
end function

function ControlIconUri(name as string) as string
    return GlyphUri("icon-" + name)
end function

function ControlIconSize() as integer
    return Px(40)
end function

function NavIconUri(destination as object, current as boolean) as string
    if current then return ControlIconUri(destination.iconOn)
    return ControlIconUri(destination.icon)
end function

function InlineIconSize() as integer
    return Px(30)
end function

function LinkIconSize() as integer
    return Px(28)
end function

function PlaceholderFor(kind as string, aspect as string) as string
    if kind = "person" then return PersonPlaceholder()
    if kind = "playlist" then return PlaylistPlaceholder()
    if aspect = LandscapeAspect() then return LandscapePlaceholder()
    return PosterPlaceholder()
end function

function NearestWidth(widths as dynamic, wanted as integer) as integer
    if type(widths) <> "roArray" or widths.Count() = 0 then return 0

    best = 0
    largest = 0
    for each width in widths
        value = Int(width)
        if value > largest then largest = value
        if value >= wanted and (best = 0 or value < best) then best = value
    end for

    if best > 0 then return best
    return largest
end function

function ImageSetUrl(serverBase as string, imageSet as dynamic, wanted as integer) as dynamic
    if imageSet = invalid then return invalid

    path = TextOrBlank(ValueAt(imageSet, "base", ""))
    if IsBlank(path) then return invalid

    width = NearestWidth(ValueAt(imageSet, "widths", invalid), wanted)
    if width = 0 then return invalid

    return JoinUrl(serverBase, path + "/" + width.ToStr())
end function

function ArtworkUrl(serverBase as string, artwork as dynamic, kind as string, wanted as integer) as dynamic
    sets = ValueAt(artwork, kind, invalid)
    if type(sets) <> "roArray" then return invalid

    for each imageSet in sets
        url = ImageSetUrl(serverBase, imageSet, wanted)
        if url <> invalid then return url
    end for
    return invalid
end function

function BackdropUrlFor(serverBase as string, artwork as dynamic) as string
    url = ArtworkUrl(serverBase, artwork, "backdrops", BackdropImageWidth())
    if url = invalid then return ""
    return url
end function

function BackdropUrlFrom(serverBase as string, candidates as dynamic) as string
    if type(candidates) <> "roArray" then return ""

    for each artwork in candidates
        for each kind in ["backdrops", "posters"]
            url = ArtworkUrl(serverBase, artwork, kind, BackdropImageWidth())
            if url <> invalid then return url
        end for
    end for
    return ""
end function

function CardImageUrl(serverBase as string, card as dynamic, wanted as integer) as string
    if card = invalid then return ""

    aspect = TextOrBlank(ValueAt(card, "aspect", PosterAspect()))
    artwork = ValueAt(card, "artwork", invalid)

    order = ["posters", "backdrops"]
    if aspect = LandscapeAspect() then order = ["backdrops", "posters"]

    for each name in order
        width = wanted
        if aspect <> LandscapeAspect() and name = "backdrops" then width = Int(wanted * PortraitBackdropScale() + 0.5)
        url = ArtworkUrl(serverBase, artwork, name, width)
        if url <> invalid then return url
    end for

    return ""
end function

function CardPlaceholderUri(card as dynamic) as string
    if card = invalid then return PosterPlaceholder()

    kind = TextOrBlank(ValueAt(card, "kind", ""))
    aspect = TextOrBlank(ValueAt(card, "aspect", PosterAspect()))
    return PlaceholderFor(kind, aspect)
end function
