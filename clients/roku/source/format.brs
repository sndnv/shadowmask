function MsToSeconds(milliseconds as integer) as float
    if milliseconds <= 0 then return 0.0
    return milliseconds / 1000.0
end function

function SecondsToMs(seconds as float) as integer
    if seconds <= 0 then return 0
    return cint(seconds * 1000)
end function

function FormatDuration(milliseconds as integer) as string
    if milliseconds <= 0 then return "0:00"

    total = cint(milliseconds / 1000)
    hours = total \ 3600
    minutes = (total - hours * 3600) \ 60
    seconds = total - hours * 3600 - minutes * 60

    if hours > 0
        return hours.ToStr() + ":" + PadTwo(minutes) + ":" + PadTwo(seconds)
    end if
    return minutes.ToStr() + ":" + PadTwo(seconds)
end function

function PadTwo(value as integer) as string
    if value < 10 then return "0" + value.ToStr()
    return value.ToStr()
end function

function ClockText(hours as integer, minutes as integer, use24 as boolean) as string
    if use24 then return PadTwo(hours) + ":" + PadTwo(minutes)

    shown = hours
    suffix = "AM"
    if hours >= 12
        suffix = "PM"
        if hours > 12 then shown = hours - 12
    end if
    if shown = 0 then shown = 12

    return shown.ToStr() + ":" + PadTwo(minutes) + " " + suffix
end function

function EpisodeCode(season as dynamic, number as dynamic) as string
    if number = invalid then return ""

    code = "E" + PadTwo(Int(number))
    if season = invalid then return code
    return "S" + PadTwo(Int(season)) + code
end function

function FormatRuntimeMinutes(minutes as dynamic) as string
    if minutes = invalid then return ""

    whole = Int(minutes)
    if whole <= 0 then return ""
    return whole.ToStr() + " min"
end function

function FormatDurationText(milliseconds as dynamic) as string
    if milliseconds = invalid then return ""

    total = Int(milliseconds / 60000.0)
    if total <= 0 then return ""

    hours = total \ 60
    minutes = total MOD 60
    if hours > 0 then return hours.ToStr() + "h " + PadTwo(minutes) + "m"
    return minutes.ToStr() + "m"
end function

function FormatMegabytes(bytes as dynamic) as string
    if bytes = invalid then return ""

    megabytes = Int(bytes / 1048576.0 + 0.5)
    if megabytes <= 0 then return ""
    return megabytes.ToStr() + " MB"
end function

function FormatFrameRate(rate as dynamic) as string
    if rate = invalid or rate <= 0 then return ""
    return Int(rate + 0.5).ToStr() + " fps"
end function

function ContentRatingLabel(rating as dynamic) as string
    system = TextOrBlank(ValueAt(rating, "system", ""))
    code = TextOrBlank(ValueAt(rating, "code", ""))
    if IsBlank(code) then return ""
    if IsBlank(system) then return UCase(code)
    return UCase(system) + " " + UCase(code)
end function

function ScoreSource(source as dynamic) as string
    text = TextOrBlank(source)
    if LCase(text) = "internet movie database" then return "IMDb"
    return text
end function

function ScoreNumber(value as dynamic) as string
    if value = invalid then return ""

    whole = Int(value)
    if value = whole then return whole.ToStr()

    scaled = Int(value * 10 + 0.5)
    return (scaled \ 10).ToStr() + "." + (scaled MOD 10).ToStr()
end function

function ScoreText(source as dynamic, value as dynamic) as string
    number = ScoreNumber(value)
    if IsBlank(number) then return ""

    suffixes = {
        "internet movie database": "/10",
        "rotten tomatoes": "%",
        "metacritic": "/100"
    }
    key = LCase(TextOrBlank(source))
    if suffixes.DoesExist(key) then return number + suffixes[key]
    return number
end function

function FormatPercent(fraction as float) as string
    if fraction <= 0 then return "0%"
    if fraction >= 1 then return "100%"
    return cint(fraction * 100).ToStr() + "%"
end function

function FormatSourceHeight(height as integer) as string
    if height >= 4320 then return "8K"
    if height >= 2160 then return "4K"
    if height >= 1440 then return "2K"
    return height.ToStr() + "p"
end function

function FormatQualityRung(height as integer) as string
    return height.ToStr() + "p"
end function

function ParseIsoDate(isoDate as string) as dynamic
    if Len(isoDate) < 10 then return invalid
    if HasUtcOffset(isoDate) then return invalid

    parsed = CreateObject("roDateTime")
    parsed.FromISO8601String(isoDate)
    if parsed.AsSeconds() = 0 then return invalid
    return parsed
end function

function HasUtcOffset(isoDate as string) as boolean
    time = Mid(isoDate, 11)
    return Instr(1, time, "+") > 0 or Instr(1, time, "-") > 0
end function

function FormatYear(isoDate as string) as string
    parsed = ParseIsoDate(isoDate)
    if parsed = invalid then return ""
    return parsed.GetYear().ToStr()
end function

function FormatDate(isoDate as string) as string
    parsed = ParseIsoDate(isoDate)
    if parsed = invalid then return ""
    return parsed.AsDateString("short-month-no-weekday")
end function
