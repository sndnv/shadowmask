function IsBlank(value as dynamic) as boolean
    if value = invalid then return true
    if type(value) <> "String" and type(value) <> "roString" then return false
    return Len(value.Trim()) = 0
end function

function TextOrBlank(value as dynamic) as string
    if value = invalid then return ""
    if type(value) = "String" or type(value) = "roString" then return value
    return ""
end function

function NumberText(value as dynamic) as string
    if value = invalid then return ""
    if type(value) = "String" or type(value) = "roString" then return value
    return Int(value).ToStr()
end function

function ValueAt(source as object, path as string, fallback = invalid as dynamic) as dynamic
    if source = invalid or IsBlank(path) then return fallback

    if Instr(1, path, ".") = 0
        if type(source) <> "roAssociativeArray" then return fallback
        if not source.DoesExist(path) then return fallback

        found = source[path]
        if found = invalid then return fallback
        return found
    end if

    current = source
    for each segment in path.Split(".")
        if type(current) <> "roAssociativeArray" then return fallback
        if not current.DoesExist(segment) then return fallback
        current = current[segment]
    end for

    if current = invalid then return fallback
    return current
end function

function FirstPresent(values as object, fallback = "" as dynamic) as dynamic
    if values = invalid then return fallback
    for each value in values
        if not IsBlank(value) then return value
    end for
    return fallback
end function

function ListItems(json as dynamic) as object
    if type(json) = "roArray" then return json

    items = ValueAt(json, "items", invalid)
    if type(items) = "roArray" then return items
    return []
end function

function ClampInt(value as integer, low as integer, high as integer) as integer
    if value < low then return low
    if value > high then return high
    return value
end function

function SplitEvenly(items as dynamic, buckets as integer) as object
    split = []
    if buckets <= 0 then return split

    for index = 1 to buckets
        split.Push([])
    end for
    if type(items) <> "roArray" then return split

    bucketed = Int((items.Count() + buckets - 1) / buckets)
    if bucketed <= 0 then return split

    at = 0
    for index = 0 to items.Count() - 1
        if split[at].Count() >= bucketed and at < buckets - 1 then at = at + 1
        split[at].Push(items[index])
    end for
    return split
end function
