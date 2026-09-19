function TitleKey(kind as dynamic, id as dynamic) as string
    return TextOrBlank(kind) + ":" + TextOrBlank(id)
end function

function CollectRefs(cards as dynamic, typeField as string, idField as string) as object
    refs = []
    if type(cards) <> "roArray" then return refs

    seen = {}
    for each card in cards
        kind = TextOrBlank(ValueAt(card, typeField, ""))
        id = TextOrBlank(ValueAt(card, idField, ""))
        if not IsBlank(kind) and not IsBlank(id)
            key = TitleKey(kind, id)
            if not seen.DoesExist(key)
                seen[key] = true
                refs.Push({ "type": kind, "id": id })
            end if
        end if
    end for
    return refs
end function

function LeafRefs(cards as dynamic) as object
    return CollectRefs(cards, "refType", "refId")
end function

function RollupTargets(cards as dynamic) as object
    return CollectRefs(cards, "rollupType", "rollupId")
end function

function TitleRefsFrom(items as dynamic) as object
    refs = []
    if type(items) <> "roArray" then return refs

    for each entry in items
        ref = ValueAt(entry, "title", invalid)
        kind = TextOrBlank(ValueAt(ref, "type", ""))
        id = TextOrBlank(ValueAt(ref, "id", ""))
        if not IsBlank(kind) and not IsBlank(id) then refs.Push({ "type": kind, "id": id })
    end for
    return refs
end function

function FirstRefs(refs as dynamic, limit as integer) as object
    if type(refs) <> "roArray" then return []
    if limit <= 0 or refs.Count() <= limit then return refs

    return refs.Slice(0, limit)
end function

function CardsForRefs(cards as dynamic, refs as dynamic) as object
    ordered = []
    if type(refs) <> "roArray" then return ordered

    lookup = {}
    if type(cards) = "roArray"
        for each card in cards
            key = TitleKey(ValueAt(card, "refType", ""), ValueAt(card, "refId", ""))
            if not lookup.DoesExist(key) then lookup[key] = card
        end for
    end if

    for each ref in refs
        key = TitleKey(ValueAt(ref, "type", ""), ValueAt(ref, "id", ""))
        if lookup.DoesExist(key) then ordered.Push(lookup[key])
    end for
    return ordered
end function

function EntriesByRef(entries as dynamic) as object
    lookup = {}
    if type(entries) <> "roArray" then return lookup

    for each entry in entries
        ref = ValueAt(entry, "title", invalid)
        kind = TextOrBlank(ValueAt(ref, "type", ""))
        id = TextOrBlank(ValueAt(ref, "id", ""))
        if not IsBlank(kind) and not IsBlank(id)
            key = TitleKey(kind, id)
            if not lookup.DoesExist(key) then lookup[key] = entry
        end if
    end for
    return lookup
end function

function CardsWithout(cards as dynamic, id as dynamic) as object
    kept = []
    if type(cards) <> "roArray" then return kept

    dropped = TextOrBlank(id)
    for each card in cards
        if TextOrBlank(ValueAt(card, "id", "")) <> dropped then kept.Push(card)
    end for
    return kept
end function

function ChunkRefs(refs as dynamic, size as integer) as object
    chunks = []
    if type(refs) <> "roArray" or refs.Count() = 0 then return chunks

    span = size
    if span <= 0 then span = MaxBatchSize()

    index = 0
    while index < refs.Count()
        endIndex = index + span
        if endIndex > refs.Count() then endIndex = refs.Count()
        chunks.Push(refs.Slice(index, endIndex))
        index = endIndex
    end while
    return chunks
end function

function StateLookup(entries as dynamic, refField as string) as object
    lookup = {}
    if type(entries) <> "roArray" then return lookup

    for each entry in entries
        ref = ValueAt(entry, refField, invalid)
        kind = TextOrBlank(ValueAt(ref, "type", ""))
        id = TextOrBlank(ValueAt(ref, "id", ""))
        if not IsBlank(kind) and not IsBlank(id) then lookup[TitleKey(kind, id)] = entry
    end for
    return lookup
end function

sub ApplyStates(cards as dynamic, states as dynamic, rollups as dynamic)
    if type(cards) <> "roArray" then return

    leaves = StateLookup(states, "title")
    targets = StateLookup(rollups, "target")

    for each card in cards
        leafKey = TitleKey(ValueAt(card, "refType", ""), ValueAt(card, "refId", ""))
        if leaves.DoesExist(leafKey)
            entry = leaves[leafKey]
            card.watched = ValueAt(entry, "watched", false) = true
            card.watchlisted = ValueAt(entry, "watchlisted", false) = true
            card.favorite = ValueAt(entry, "favorite", false) = true
            percent = Int(ValueAt(entry, "progress_percent", 0))
            if percent > 0 then card.progressPercent = percent
        end if

        rollupKey = TitleKey(ValueAt(card, "rollupType", ""), ValueAt(card, "rollupId", ""))
        if targets.DoesExist(rollupKey)
            card.watched = ValueAt(targets[rollupKey], "watched", false) = true
        end if
    end for

end sub

function CardStateList(cards as dynamic) as object
    states = []
    if type(cards) <> "roArray" then return states

    for each card in cards
        states.Push({
            watched: ValueAt(card, "watched", false) = true,
            progressPercent: Int(ValueAt(card, "progressPercent", 0))
        })
    end for
    return states
end function

function StateWindow(states as integer, start as integer, nodes as integer, cards as integer) as integer
    count = states
    if nodes - start < count then count = nodes - start
    if cards - start < count then count = cards - start
    if count < 0 then count = 0
    return count
end function
