//! Friendly names for locale codes.
//!
//! The installer offers every locale the system reports (`localectl list-locales`),
//! but shows them as "Language (Country)" rather than raw codes like `en_US.UTF-8`.
//! Language names are given natively (e.g. `Русский`); countries in English. Codes
//! not in these tables fall back to the raw code, which stays searchable.

/// Language code → native name.
const LANGUAGES: &[(&str, &str)] = &[
    ("aa", "Afar"), ("af", "Afrikaans"), ("ak", "Akan"), ("am", "አማርኛ"),
    ("ar", "العربية"), ("as", "অসমীয়া"), ("az", "Azərbaycan"), ("be", "Беларуская"),
    ("bg", "Български"), ("bn", "বাংলা"), ("bo", "བོད་སྐད་"), ("br", "Brezhoneg"),
    ("bs", "Bosanski"), ("ca", "Català"), ("cs", "Čeština"), ("cy", "Cymraeg"),
    ("da", "Dansk"), ("de", "Deutsch"), ("dz", "རྫོང་ཁ"), ("el", "Ελληνικά"),
    ("en", "English"), ("eo", "Esperanto"), ("es", "Español"), ("et", "Eesti"),
    ("eu", "Euskara"), ("fa", "فارسی"), ("fi", "Suomi"), ("fo", "Føroyskt"),
    ("fr", "Français"), ("ga", "Gaeilge"), ("gd", "Gàidhlig"), ("gl", "Galego"),
    ("gu", "ગુજરાતી"), ("he", "עברית"), ("hi", "हिन्दी"), ("hr", "Hrvatski"),
    ("hu", "Magyar"), ("hy", "Հայերեն"), ("id", "Indonesia"), ("is", "Íslenska"),
    ("it", "Italiano"), ("ja", "日本語"), ("ka", "ქართული"), ("kk", "Қазақ"),
    ("km", "ខ្មែរ"), ("kn", "ಕನ್ನಡ"), ("ko", "한국어"), ("ku", "Kurdî"),
    ("ky", "Кыргызча"), ("lo", "ລາວ"), ("lt", "Lietuvių"), ("lv", "Latviešu"),
    ("mk", "Македонски"), ("ml", "മലയാളം"), ("mn", "Монгол"), ("mr", "मराठी"),
    ("ms", "Melayu"), ("mt", "Malti"), ("my", "မြန်မာ"), ("nb", "Norsk bokmål"),
    ("ne", "नेपाली"), ("nl", "Nederlands"), ("nn", "Norsk nynorsk"), ("no", "Norsk"),
    ("or", "ଓଡ଼ିଆ"), ("pa", "ਪੰਜਾਬੀ"), ("pl", "Polski"), ("ps", "پښتو"),
    ("pt", "Português"), ("ro", "Română"), ("ru", "Русский"), ("si", "සිංහල"),
    ("sk", "Slovenčina"), ("sl", "Slovenščina"), ("sq", "Shqip"), ("sr", "Српски"),
    ("sv", "Svenska"), ("sw", "Kiswahili"), ("ta", "தமிழ்"), ("te", "తెలుగు"),
    ("tg", "Тоҷикӣ"), ("th", "ไทย"), ("ti", "ትግርኛ"), ("tk", "Türkmen"),
    ("tr", "Türkçe"), ("tt", "Татар"), ("ug", "ئۇيغۇرچە"), ("uk", "Українська"),
    ("ur", "اردو"), ("uz", "Oʻzbek"), ("vi", "Tiếng Việt"), ("wa", "Walon"),
    ("xh", "isiXhosa"), ("yi", "ייִדיש"), ("zh", "中文"), ("zu", "isiZulu"),
];

/// Country code → English name.
const COUNTRIES: &[(&str, &str)] = &[
    ("AD", "Andorra"), ("AE", "United Arab Emirates"), ("AF", "Afghanistan"),
    ("AL", "Albania"), ("AM", "Armenia"), ("AR", "Argentina"), ("AT", "Austria"),
    ("AU", "Australia"), ("AZ", "Azerbaijan"), ("BA", "Bosnia and Herzegovina"),
    ("BD", "Bangladesh"), ("BE", "Belgium"), ("BG", "Bulgaria"), ("BH", "Bahrain"),
    ("BO", "Bolivia"), ("BR", "Brazil"), ("BT", "Bhutan"), ("BY", "Belarus"),
    ("CA", "Canada"), ("CH", "Switzerland"), ("CL", "Chile"), ("CN", "China"),
    ("CO", "Colombia"), ("CR", "Costa Rica"), ("CY", "Cyprus"), ("CZ", "Czechia"),
    ("DE", "Germany"), ("DK", "Denmark"), ("DO", "Dominican Republic"), ("DZ", "Algeria"),
    ("EC", "Ecuador"), ("EE", "Estonia"), ("EG", "Egypt"), ("ER", "Eritrea"),
    ("ES", "Spain"), ("ET", "Ethiopia"), ("FI", "Finland"), ("FR", "France"),
    ("GB", "United Kingdom"), ("GE", "Georgia"), ("GH", "Ghana"), ("GR", "Greece"),
    ("GT", "Guatemala"), ("HK", "Hong Kong"), ("HN", "Honduras"), ("HR", "Croatia"),
    ("HU", "Hungary"), ("ID", "Indonesia"), ("IE", "Ireland"), ("IL", "Israel"),
    ("IN", "India"), ("IQ", "Iraq"), ("IR", "Iran"), ("IS", "Iceland"),
    ("IT", "Italy"), ("JO", "Jordan"), ("JP", "Japan"), ("KE", "Kenya"),
    ("KH", "Cambodia"), ("KR", "South Korea"), ("KW", "Kuwait"), ("KZ", "Kazakhstan"),
    ("LA", "Laos"), ("LB", "Lebanon"), ("LK", "Sri Lanka"), ("LT", "Lithuania"),
    ("LU", "Luxembourg"), ("LV", "Latvia"), ("LY", "Libya"), ("MA", "Morocco"),
    ("MD", "Moldova"), ("ME", "Montenegro"), ("MK", "North Macedonia"), ("MM", "Myanmar"),
    ("MN", "Mongolia"), ("MT", "Malta"), ("MX", "Mexico"), ("MY", "Malaysia"),
    ("NG", "Nigeria"), ("NI", "Nicaragua"), ("NL", "Netherlands"), ("NO", "Norway"),
    ("NP", "Nepal"), ("NZ", "New Zealand"), ("OM", "Oman"), ("PA", "Panama"),
    ("PE", "Peru"), ("PH", "Philippines"), ("PK", "Pakistan"), ("PL", "Poland"),
    ("PR", "Puerto Rico"), ("PT", "Portugal"), ("PY", "Paraguay"), ("QA", "Qatar"),
    ("RO", "Romania"), ("RS", "Serbia"), ("RU", "Russia"), ("SA", "Saudi Arabia"),
    ("SE", "Sweden"), ("SG", "Singapore"), ("SI", "Slovenia"), ("SK", "Slovakia"),
    ("SV", "El Salvador"), ("SY", "Syria"), ("TH", "Thailand"), ("TJ", "Tajikistan"),
    ("TM", "Turkmenistan"), ("TN", "Tunisia"), ("TR", "Türkiye"), ("TW", "Taiwan"),
    ("UA", "Ukraine"), ("US", "United States"), ("UY", "Uruguay"), ("UZ", "Uzbekistan"),
    ("VE", "Venezuela"), ("VN", "Vietnam"), ("YE", "Yemen"), ("ZA", "South Africa"),
    ("ZW", "Zimbabwe"),
];

fn lookup(table: &[(&'static str, &'static str)], key: &str) -> Option<&'static str> {
    table.iter().find(|(code, _)| *code == key).map(|(_, name)| *name)
}

/// Turn a locale code into "Language (Country)", with a trailing modifier note.
/// e.g. `en_US.UTF-8` → `English (United States)`; `sr_RS.UTF-8@latin` →
/// `Српски (Serbia, latin)`. Unknown codes fall back to the raw pieces.
pub fn friendly(locale: &str) -> String {
    // Strip the charset (".UTF-8") but keep a trailing "@modifier".
    let (before_charset, modifier) = match locale.split_once('@') {
        Some((head, m)) => (head.split('.').next().unwrap_or(head), Some(m)),
        None => (locale.split('.').next().unwrap_or(locale), None),
    };
    let (lang_code, country_code) = before_charset.split_once('_').unwrap_or((before_charset, ""));

    let lang = lookup(LANGUAGES, lang_code).unwrap_or(lang_code);
    let mut out = lang.to_string();
    if !country_code.is_empty() {
        let country = lookup(COUNTRIES, country_code).unwrap_or(country_code);
        match modifier {
            Some(m) => out.push_str(&format!(" ({country}, {m})")),
            None => out.push_str(&format!(" ({country})")),
        }
    } else if let Some(m) = modifier {
        out.push_str(&format!(" ({m})"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_names() {
        assert_eq!(friendly("en_US.UTF-8"), "English (United States)");
        assert_eq!(friendly("ru_RU.UTF-8"), "Русский (Russia)");
        assert_eq!(friendly("pt_BR.UTF-8"), "Português (Brazil)");
        assert_eq!(friendly("sr_RS.UTF-8@latin"), "Српски (Serbia, latin)");
    }

    #[test]
    fn unknown_codes_fall_back() {
        assert_eq!(friendly("xx_YY.UTF-8"), "xx (YY)");
    }
}
