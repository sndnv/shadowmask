function SubtitleCues(text as string) as object
    parser = {
        timing: CreateObject("roRegex", "^\s*((?:\d+:)?\d{2}:\d{2}\.\d{3})\s+-->\s+((?:\d+:)?\d{2}:\d{2}\.\d{3})(?:\s|$)", ""),
        tag: CreateObject("roRegex", "^/?([A-Za-z]+)", "")
    }

    cues = []
    block = []
    lines = SubtitleSourceLines(text)
    lines.Push("")
    for each line in lines
        if line.Trim() = ""
            cue = SubtitleCueOf(block, parser)
            if cue <> invalid then SubtitleCueInserted(cues, cue)
            block = []
        else
            block.Push(line)
        end if
    end for

    reach = 0
    for each cue in cues
        if cue.endMs > reach then reach = cue.endMs
        cue.reachMs = reach
    end for
    return cues
end function

sub SubtitleCueInserted(cues as object, cue as object)
    at = cues.Count()
    while at > 0
        if cues[at - 1].startMs <= cue.startMs then exit while
        at = at - 1
    end while

    cues.Push(cue)
    index = cues.Count() - 1
    while index > at
        cues[index] = cues[index - 1]
        index = index - 1
    end while
    cues[at] = cue
end sub

function SubtitleSourceLines(text as string) as object
    body = text
    if Left(body, 1) = Chr(65279) then body = Mid(body, 2)
    body = body.Replace(Chr(13) + Chr(10), Chr(10)).Replace(Chr(13), Chr(10))
    return body.Split(Chr(10))
end function

function SubtitleCueOf(block as object, parser as object) as dynamic
    if block.Count() = 0 then return invalid

    first = block[0]
    for each keyword in ["NOTE", "STYLE", "REGION"]
        if first = keyword or Left(first, Len(keyword) + 1) = keyword + " " or Left(first, Len(keyword) + 1) = keyword + Chr(9) then return invalid
    end for

    at = 0
    if Instr(1, first, "-->") = 0 then at = 1
    if at >= block.Count() then return invalid

    found = parser.timing.Match(block[at])
    if found.Count() < 3 then return invalid
    startMs = SubtitleTimeMs(found[1])
    endMs = SubtitleTimeMs(found[2])
    if endMs <= startMs then return invalid

    lines = []
    depth = { i: 0, b: 0, u: 0 }
    for index = at + 1 to block.Count() - 1
        line = SubtitleLine(block[index], parser, depth)
        if line <> invalid then lines.Push(line)
    end for
    if lines.Count() = 0 then return invalid

    return { startMs: startMs, endMs: endMs, reachMs: endMs, lines: lines }
end function

function SubtitleTimeMs(stamp as string) as integer
    parts = stamp.Split(":")
    last = parts.Count() - 1
    seconds = parts[last].Split(".")
    total = seconds[0].ToInt() * 1000 + seconds[1].ToInt() + parts[last - 1].ToInt() * 60000
    if last = 2 then total = total + parts[0].ToInt() * 3600000
    return total
end function

function SubtitleLine(raw as string, parser as object, depth as object) as dynamic
    whole = { i: true, b: true, u: true }
    text = ""
    rest = raw
    while rest <> ""
        open = Instr(1, rest, "<")
        close = 0
        if open > 0 then close = Instr(open, rest, ">")

        piece = rest
        tag = ""
        if close > 0
            piece = Left(rest, open - 1)
            tag = Mid(rest, open + 1, close - open - 1)
            rest = Mid(rest, close + 1)
        else
            rest = ""
        end if

        shown = SubtitleEntities(piece)
        if shown.Trim() <> ""
            for each style in depth
                if depth[style] = 0 then whole[style] = false
            end for
        end if
        text = text + shown
        SubtitleTagStep(depth, tag, parser)
    end while

    text = text.Trim()
    if text = "" then return invalid
    return { text: text, italic: whole.i, bold: whole.b, underline: whole.u }
end function

sub SubtitleTagStep(depth as object, tag as string, parser as object)
    found = parser.tag.Match(tag)
    if found.Count() < 2 then return

    name = LCase(found[1])
    if not depth.DoesExist(name) then return

    if Left(tag, 1) <> "/"
        depth[name] = depth[name] + 1
    else if depth[name] > 0
        depth[name] = depth[name] - 1
    end if
end sub

function SubtitleEntities(piece as string) as string
    if Instr(1, piece, "&") = 0 then return piece
    text = piece.Replace("&lt;", "<").Replace("&gt;", ">").Replace("&nbsp;", Chr(160)).Replace("&lrm;", "").Replace("&rlm;", "")
    return text.Replace("&amp;", "&")
end function

function SubtitleFontRole(line as object, italicReady as boolean) as string
    if line.italic = true and italicReady and ItalicDrawable(line.text) then return "italic"
    if line.bold = true then return "medium"
    return "regular"
end function

function ItalicDrawable(text as string) as boolean
    for index = 1 to Len(text)
        if not ItalicCovers(Asc(Mid(text, index, 1))) then return false
    end for
    return true
end function

function ItalicCovers(code as integer) as boolean
    for each range in ItalicRanges()
        if code >= range[0] and code <= range[1] then return true
    end for
    return false
end function

function ItalicRanges() as object
    return [
        [&h20, &h7E], [&hA0, &h377], [&h37A, &h37F], [&h384, &h38A], [&h38C, &h38C], [&h38E, &h3A1], [&h3A3, &h3E1],
        [&h3F0, &h52F], [&h1E00, &h1EFF], [&h2000, &h2064], [&h2066, &h2071], [&h2074, &h208E], [&h2090, &h209C],
        [&h20A0, &h20BF], [&h2100, &h215F], [&h2212, &h2212]
    ]
end function

function ActiveCues(cues as object, atMs as integer) as object
    low = 0
    high = cues.Count() - 1
    last = -1
    while low <= high
        middle = (low + high) \ 2
        if cues[middle].startMs <= atMs
            last = middle
            low = middle + 1
        else
            high = middle - 1
        end if
    end while

    shown = []
    index = last
    while index >= 0
        if cues[index].reachMs <= atMs then exit while
        if cues[index].endMs > atMs then shown.Unshift(index)
        index = index - 1
    end while
    return shown
end function

function SubtitleClockHeld(positionMs as integer, nowMs as integer, playing as boolean) as object
    return { positionMs: positionMs, atMs: nowMs, playing: playing, holding: true, heldAtMs: nowMs }
end function

function SubtitleClockAt(clock as object, nowMs as integer) as integer
    if clock.holding or not clock.playing then return clock.positionMs

    elapsed = nowMs - clock.atMs
    if elapsed < 0 then elapsed = 0
    if elapsed > SubtitleClockLeadMs() then elapsed = SubtitleClockLeadMs()
    return clock.positionMs + elapsed
end function

function SubtitleClockLeadMs() as integer
    return 2000
end function

function SubtitleClockRead(clock as object, positionMs as integer, nowMs as integer) as object
    if clock.holding and Abs(positionMs - clock.positionMs) > SeekSettleMs() and nowMs - clock.heldAtMs < SeekSettleWaitMs() then return clock
    return { positionMs: positionMs, atMs: nowMs, playing: clock.playing, holding: false, heldAtMs: clock.heldAtMs }
end function

function SubtitleClockState(clock as object, state as string, nowMs as integer) as object
    return { positionMs: SubtitleClockAt(clock, nowMs), atMs: nowMs, playing: state = "playing", holding: clock.holding, heldAtMs: clock.heldAtMs }
end function

function SubtitleTickSeconds() as float
    return 0.1
end function

function SubtitleFetchSeconds() as integer
    return 20
end function

function SubtitlesOnScreen(painted as boolean, state as string) as boolean
    return painted and state <> "finished"
end function

function SameIndexes(left as object, right as object) as boolean
    if left.Count() <> right.Count() then return false
    for index = 0 to left.Count() - 1
        if left[index] <> right[index] then return false
    end for
    return true
end function

function SubtitleFileUrl(stream as dynamic, captions as boolean) as string
    if not captions or stream = invalid then return ""
    if TextOrBlank(ValueAt(stream, "subtitleDelivery", "")) <> "external" then return ""
    return TextOrBlank(ValueAt(stream, "subtitleUrl", ""))
end function
