sub init()
    m.top.functionName = "execute"
end sub

sub execute()
    job = m.top.job
    fs = CreateObject("roFileSystem")
    fs.CreateDirectory(job.directory)

    for each path in job.discard
        fs.Delete(path)
    end for

    wanted = {}
    for each file in job.files
        wanted[file.path] = true
    end for
    for each name in fs.GetDirectoryListing(job.directory)
        path = job.directory + "/" + name
        if not wanted.DoesExist(path) then fs.Delete(path)
    end for

    notes = []
    missing = []
    for each file in job.files
        stat = fs.Stat(file.path)
        if stat = invalid or stat.size <> file.size
            missing.Push(file)
        else
            notes.Push(file.role + " kept")
        end if
    end for
    m.top.count = missing.Count()

    for index = 0 to missing.Count() - 1
        file = missing[index]
        m.top.step = index + 1
        failure = Download(fs, file, job)
        if failure <> ""
            notes.Push(file.role + " failed: " + failure)
            print "[fonts] "; notes.Join(", ")
            m.top.result = { ok: false, error: failure }
            return
        end if
        notes.Push(file.role + " downloaded")
    end for

    print "[fonts] "; notes.Join(", ")
    m.top.result = { ok: true, error: "" }
end sub

function Download(fs as object, file as object, job as object) as string
    part = file.part
    fs.Delete(part)

    port = CreateObject("roMessagePort")
    transfer = CreateObject("roUrlTransfer")
    transfer.SetMessagePort(port)
    transfer.SetUrl(file.url)
    if Len(job.certificates) > 0
        transfer.SetCertificatesFile(job.certificates)
        transfer.InitClientCertificates()
    end if

    if not transfer.AsyncGetToFile(part) then return "could not start the download"

    message = wait(job.timeout * 1000, port)
    if type(message) <> "roUrlEvent"
        transfer.AsyncCancel()
        fs.Delete(part)
        return "timed out"
    end if

    code = message.GetResponseCode()
    if code <> 200
        fs.Delete(part)
        reason = message.GetFailureReason()
        if Len(reason) = 0 then reason = "HTTP " + code.ToStr()
        return reason
    end if

    stat = fs.Stat(part)
    if stat = invalid or stat.size <> file.size
        fs.Delete(part)
        return "the download was incomplete"
    end if

    if Sha256Of(part, file.size, job.chunk) <> file.sha256
        fs.Delete(part)
        return "the download did not match its checksum"
    end if

    fs.Delete(file.path)
    if not fs.Rename(part, file.path)
        fs.Delete(part)
        return "the file could not be saved"
    end if
    return ""
end function

function Sha256Of(path as string, size as dynamic, chunk as integer) as string
    digest = CreateObject("roEVPDigest")
    digest.Setup("sha256")
    bytes = CreateObject("roByteArray")
    offset = 0
    while offset < size
        length = chunk
        if size - offset < length then length = size - offset
        if not bytes.ReadFile(path, offset, length) then return ""
        digest.Update(bytes)
        offset = offset + length
    end while
    return LCase(digest.Final())
end function
