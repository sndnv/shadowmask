sub init()
    m.backdrop = m.top.FindNode("backdrop")
    m.edge = m.top.FindNode("edge")
    m.brand = m.top.FindNode("brand")
    m.list = m.top.FindNode("list")
    m.identity = m.top.FindNode("identity")

    m.list.ObserveField("itemSelected", "onSelected")
end sub

function NavDestinations() as object
    return [
        { id: "HomeScreen", label: Phrase("nav.home") },
        { id: "MoviesScreen", label: Phrase("nav.movies") },
        { id: "SeriesScreen", label: Phrase("nav.series") },
        { id: "CollectionsScreen", label: Phrase("nav.collections") },
        { id: "SearchScreen", label: Phrase("nav.search") },
        { id: "AccountScreen", label: Phrase("nav.account") }
    ]
end function

function NavIndexFor(section as string) as integer
    destinations = NavDestinations()
    for index = 0 to destinations.Count() - 1
        if destinations[index].id = section then return index
    end for
    return 0
end function

sub render()
    theme = m.top.theme
    if theme = invalid or theme.Count() = 0 then return

    sizes = TypeScale()
    space = SpacingScale()
    width = m.top.railWidth

    m.backdrop.width = width
    m.backdrop.height = CanvasHeight()
    m.backdrop.color = theme.surface

    m.edge.width = 2
    m.edge.height = CanvasHeight()
    m.edge.color = theme.border
    m.edge.translation = [width - 2, 0]

    m.brand.uri = "pkg:/images/brand-mark.png"
    m.brand.width = space.s8
    m.brand.height = space.s8
    m.brand.blendColor = theme.accent
    m.brand.translation = [space.s5, space.s6]

    rowHeight = Int(sizes.textLg * 2.2)
    m.list.itemSize = [width - space.s5 * 2, rowHeight]
    m.list.numRows = NavDestinations().Count()
    m.list.translation = [space.s5, space.s6 + space.s8 + space.s6]
    m.list.font = SizedFont(sizes.textLg)
    m.list.color = theme.muted
    m.list.focusedColor = theme.accentContrast
    m.list.focusedFont = SizedBoldFont(sizes.textLg)
    m.list.focusBitmapBlendColor = theme.accent
    m.list.vertFocusAnimationStyle = "fixedFocusWrap"

    m.identity.color = theme.muted
    m.identity.font = SizedFont(sizes.textSm)
    m.identity.width = width - space.s5 * 2
    m.identity.maxLines = 1
    m.identity.ellipsisText = "…"
    m.identity.translation = [space.s5, CanvasHeight() - space.s8]

    RefreshContent()
end sub

sub RefreshContent()
    root = CreateObject("roSGNode", "ContentNode")
    for each destination in NavDestinations()
        row = root.CreateChild("ContentNode")
        row.title = destination.label
    end for
    m.list.content = root

    m.identity.text = m.top.identity
    ApplySection()
end sub

sub ApplySection()
    if m.list.content = invalid then return
    m.list.jumpToItem = NavIndexFor(m.top.section)
end sub

sub onSelected()
    destinations = NavDestinations()
    index = ClampInt(m.list.itemSelected, 0, destinations.Count() - 1)
    m.top.selected = destinations[index].id
end sub

sub onFocus()
    if m.top.hasFocus() then m.list.SetFocus(true)
end sub

function onKeyEvent(key as string, press as boolean) as boolean
    if not m.top.visible or not KeysAreOurs() then return false
    if not press then return false

    if key = "right" or key = "back"
        m.top.dismissed = not m.top.dismissed
        return true
    end if

    if key = "left" then return true

    return false
end function
