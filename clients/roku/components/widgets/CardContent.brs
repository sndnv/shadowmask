function CardContentNode(card as object, serverUrl as string, imageWidth as integer, captionRows = 3 as integer) as object
    node = CreateObject("roSGNode", "ContentNode")

    node.AddFields({
        audioGuideText: "",
        captionRows: captionRows,
        cardTitle: TextOrBlank(ValueAt(card, "title", "")),
        cardSubtitle: TextOrBlank(ValueAt(card, "subtitle", "")),
        cardCaption: TextOrBlank(ValueAt(card, "caption", "")),
        cardKind: TextOrBlank(ValueAt(card, "kind", "")),
        cardId: TextOrBlank(ValueAt(card, "id", "")),
        seriesId: TextOrBlank(ValueAt(card, "seriesId", "")),
        seasonId: TextOrBlank(ValueAt(card, "seasonId", "")),
        versionId: TextOrBlank(ValueAt(card, "versionId", "")),
        aspect: TextOrBlank(ValueAt(card, "aspect", PosterAspect())),
        imageUri: CardImageUrl(serverUrl, card, imageWidth),
        placeholderUri: CardPlaceholderUri(card),
        backdropUri: BackdropUrlFor(serverUrl, ValueAt(card, "artwork", invalid)),
        watched: ValueAt(card, "watched", false) = true,
        watchlisted: ValueAt(card, "watchlisted", false) = true,
        favorite: ValueAt(card, "favorite", false) = true,
        dismissible: ValueAt(card, "dismissible", false) = true,
        plain: ValueAt(card, "plain", false) = true,
        badge: TextOrBlank(ValueAt(card, "badge", "")),
        progressPercent: Int(ValueAt(card, "progressPercent", 0))
    })

    node.title = TextOrBlank(ValueAt(card, "title", ""))
    node.audioGuideText = CardSpeech(card)

    return node
end function

function CardContentList(cards as object, serverUrl as string, imageWidth as integer, captionRows = 3 as integer) as object
    root = CreateObject("roSGNode", "ContentNode")
    if type(cards) <> "roArray" then return root

    for each card in cards
        root.AppendChild(CardContentNode(card, serverUrl, imageWidth, captionRows))
    end for
    return root
end function

function CardTarget(content as dynamic) as object
    if content = invalid then return {}

    return {
        kind: content.cardKind,
        id: content.cardId,
        title: content.cardTitle,
        subtitle: content.cardSubtitle,
        caption: content.cardCaption,
        imageUri: content.imageUri,
        backdropUri: content.backdropUri,
        aspect: content.aspect,
        seriesId: content.seriesId,
        seasonId: content.seasonId,
        versionId: content.versionId,
        dismissible: content.dismissible,
        watched: content.watched,
        watchlisted: content.watchlisted,
        favorite: content.favorite,
        progressPercent: content.progressPercent
    }
end function
