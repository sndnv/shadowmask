function LanguageTable() as object
    if m.DoesExist("smLanguages") then return m.smLanguages

    m.smLanguages = [
        { code: "af", name: "Afrikaans" },
        { code: "sq", name: "Albanian" },
        { code: "am", name: "Amharic" },
        { code: "ar", name: "Arabic" },
        { code: "hy", name: "Armenian" },
        { code: "az", name: "Azerbaijani" },
        { code: "eu", name: "Basque" },
        { code: "be", name: "Belarusian" },
        { code: "bn", name: "Bengali" },
        { code: "bs", name: "Bosnian" },
        { code: "bg", name: "Bulgarian" },
        { code: "my", name: "Burmese" },
        { code: "ca", name: "Catalan" },
        { code: "zh", name: "Chinese" },
        { code: "hr", name: "Croatian" },
        { code: "cs", name: "Czech" },
        { code: "da", name: "Danish" },
        { code: "nl", name: "Dutch" },
        { code: "en", name: "English" },
        { code: "eo", name: "Esperanto" },
        { code: "et", name: "Estonian" },
        { code: "fil", name: "Filipino" },
        { code: "fi", name: "Finnish" },
        { code: "fr", name: "French" },
        { code: "gl", name: "Galician" },
        { code: "ka", name: "Georgian" },
        { code: "de", name: "German" },
        { code: "el", name: "Greek" },
        { code: "he", name: "Hebrew" },
        { code: "hi", name: "Hindi" },
        { code: "hu", name: "Hungarian" },
        { code: "is", name: "Icelandic" },
        { code: "id", name: "Indonesian" },
        { code: "ga", name: "Irish" },
        { code: "it", name: "Italian" },
        { code: "ja", name: "Japanese" },
        { code: "kn", name: "Kannada" },
        { code: "kk", name: "Kazakh" },
        { code: "km", name: "Khmer" },
        { code: "ko", name: "Korean" },
        { code: "ku", name: "Kurdish" },
        { code: "lo", name: "Lao" },
        { code: "la", name: "Latin" },
        { code: "lv", name: "Latvian" },
        { code: "lt", name: "Lithuanian" },
        { code: "mk", name: "Macedonian" },
        { code: "ms", name: "Malay" },
        { code: "ml", name: "Malayalam" },
        { code: "mt", name: "Maltese" },
        { code: "mr", name: "Marathi" },
        { code: "mn", name: "Mongolian" },
        { code: "ne", name: "Nepali" },
        { code: "no", name: "Norwegian" },
        { code: "fa", name: "Persian" },
        { code: "pl", name: "Polish" },
        { code: "pt", name: "Portuguese" },
        { code: "pa", name: "Punjabi" },
        { code: "ro", name: "Romanian" },
        { code: "ru", name: "Russian" },
        { code: "sr", name: "Serbian" },
        { code: "si", name: "Sinhala" },
        { code: "sk", name: "Slovak" },
        { code: "sl", name: "Slovenian" },
        { code: "so", name: "Somali" },
        { code: "es", name: "Spanish" },
        { code: "sw", name: "Swahili" },
        { code: "sv", name: "Swedish" },
        { code: "tl", name: "Tagalog" },
        { code: "ta", name: "Tamil" },
        { code: "te", name: "Telugu" },
        { code: "th", name: "Thai" },
        { code: "tr", name: "Turkish" },
        { code: "uk", name: "Ukrainian" },
        { code: "ur", name: "Urdu" },
        { code: "uz", name: "Uzbek" },
        { code: "vi", name: "Vietnamese" },
        { code: "cy", name: "Welsh" },
        { code: "yi", name: "Yiddish" },
        { code: "zu", name: "Zulu" }
    ]
    return m.smLanguages
end function

function KnownLanguage(code as dynamic) as dynamic
    wanted = LCase(TextOrBlank(code))
    if IsBlank(wanted) then return invalid

    for each entry in LanguageTable()
        if entry.code = wanted then return entry
    end for
    return invalid
end function

function LanguageName(code as dynamic) as string
    entry = KnownLanguage(code)
    if entry <> invalid then return entry.name
    return UCase(TextOrBlank(code))
end function

function VersionLanguages(version as dynamic) as object
    codes = []
    seen = {}
    for each field in ["audio", "subtitles", "subtitle_files"]
        tracks = ValueAt(version, field, invalid)
        if type(tracks) = "roArray"
            for each track in tracks
                entry = KnownLanguage(ValueAt(track, "language", ""))
                if entry <> invalid and not seen.DoesExist(entry.code)
                    seen[entry.code] = true
                    codes.Push(entry.code)
                end if
            end for
        end if
    end for
    return codes
end function

function LanguageOptions(version as dynamic) as object
    options = []
    seen = {}
    for each code in VersionLanguages(version)
        seen[code] = true
        options.Push({ code: code, label: LanguageName(code), detail: UCase(code) })
    end for

    for each entry in LanguageTable()
        if not seen.DoesExist(entry.code) then options.Push({ code: entry.code, label: entry.name, detail: UCase(entry.code) })
    end for
    return options
end function
