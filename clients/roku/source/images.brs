function CardImageWidthFor(cardWidth as dynamic) as integer
    if cardWidth = invalid then return RailCardWidth()

    width = Int(cardWidth)
    if width <= 0 then return RailCardWidth()
    return width
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

function PlaceholderGlyphSize() as integer
    return 96
end function

function GlyphUri(name as string) as string
    if IsBlank(name) then return ""
    return "pkg:/images/" + name + ".png"
end function

function ChevronUri(direction as string) as string
    return GlyphUri("chevron-" + direction)
end function

function ChevronSize() as integer
    return 44
end function

function RoundedFillUri() as string
    return "pkg:/images/button-fill.9.png"
end function

function RoundedLineUri() as string
    return "pkg:/images/button-line.9.png"
end function

function ChipCapUri(side as string) as string
    return GlyphUri("chip-cap-" + side)
end function

function ChipCapLineUri(side as string) as string
    return GlyphUri("chip-cap-" + side + "-line")
end function

function ControlIconUri(name as string) as string
    return GlyphUri("icon-" + name)
end function

function ControlIconSize() as integer
    return 40
end function

function LinkIconSize() as integer
    return 28
end function

function PlaceholderFor(kind as string, aspect as string) as string
    if kind = "person" then return PersonPlaceholder()
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
        url = ArtworkUrl(serverBase, artwork, name, wanted)
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
