//! Korean commands put the verb last: "회의 연기 메일 써줘", "날씨 검색해줘",
//! "이 문장 영어로 번역해줘". Matching therefore looks at how the request ends.

use super::{CommandMatch, SearchMatch};
use crate::voice_intent::normalize::NormalizedUtterance;
use crate::voice_intent::SearchProvider;

/// Words that point at the selected text; rewrite and translation commands start with one.
const SELECTION_WORDS: [&str; 12] = [
    "이거",
    "이것",
    "이걸",
    "이건",
    "이 부분",
    "이 문장",
    "이 문단",
    "이 글",
    "이 내용",
    "선택한",
    "선택된",
    "여기",
];

/// Things a draft command can ask for. A request to write must name one, so ordinary
/// sentences that happen to end in "써줘" stay dictation.
const DRAFT_ARTIFACTS: [&str; 16] = [
    "메일",
    "이메일",
    "답장",
    "답변",
    "회신",
    "메시지",
    "문자",
    "편지",
    "공지",
    "안내문",
    "초대",
    "초안",
    "보고서",
    "제안서",
    "축하",
    "감사 인사",
];

/// The command without closing punctuation.
fn body<'a>(view: &'a NormalizedUtterance<'_>) -> &'a str {
    view.match_text()
        .trim_end_matches(['.', '!', '?', '。', '！', '？', '~', ' '])
}

/// Every polite or plain way to ask for `stems`: "검색해" → "검색해줘", "검색해 주세요", ...
fn requests(stems: &[&str]) -> Vec<String> {
    const ENDINGS: [&str; 10] = [
        "줘",
        " 줘",
        "주세요",
        " 주세요",
        "줄래",
        " 줄래",
        "줄래요",
        " 줄래요",
        "줘요",
        " 줘요",
    ];
    stems
        .iter()
        .flat_map(|stem| ENDINGS.iter().map(move |ending| format!("{stem}{ending}")))
        .collect()
}

/// Where the longest of `endings` starts in `text`, if `text` ends with one.
fn ending_start(text: &str, endings: &[String]) -> Option<usize> {
    endings
        .iter()
        .filter_map(|ending| text.strip_suffix(ending.as_str()).map(str::len))
        .min()
}

fn starts_with_selection_word(text: &str) -> bool {
    SELECTION_WORDS.iter().any(|word| {
        text.strip_prefix(word)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
    })
}

pub(super) fn match_draft(view: &NormalizedUtterance<'_>) -> CommandMatch<String> {
    let text = body(view);
    let Some(end) = ending_start(text, &requests(&["써", "작성해", "작성 해", "적어"]))
    else {
        return CommandMatch::NoMatch;
    };
    let request = text[..end].trim_end();
    if !DRAFT_ARTIFACTS
        .iter()
        .any(|artifact| request.contains(artifact))
    {
        return CommandMatch::NoMatch;
    }
    view.original_for_match_range(0, request.len())
        .map(CommandMatch::Matched)
        .unwrap_or(CommandMatch::MissingPayload)
}

pub(super) fn matches_rewrite(view: &NormalizedUtterance<'_>) -> bool {
    let text = body(view);
    starts_with_selection_word(text)
        && ending_start(
            text,
            &requests(&[
                "다시 써",
                "다시 작성해",
                "고쳐",
                "다듬어",
                "바꿔",
                "줄여",
                "늘려",
                "게 해",
                "게 써",
                "게 만들어",
            ]),
        )
        .is_some()
}

pub(super) fn matches_translation(view: &NormalizedUtterance<'_>) -> bool {
    let text = body(view);
    if !starts_with_selection_word(text) {
        return false;
    }
    let Some(end) = ending_start(text, &requests(&["번역해", "번역 해", "바꿔", "옮겨"]))
    else {
        return false;
    };
    // The target language comes right before the verb: "영어로", "일본어로".
    text[..end].trim_end().ends_with('로')
}

pub(super) fn matches_informational(view: &NormalizedUtterance<'_>) -> bool {
    let text = body(view);
    ending_start(
        text,
        &requests(&["요약해", "설명해", "비교해", "알려", "정리해"]),
    )
    .is_some()
        || [
            "무슨 뜻",
            "무슨 의미",
            "뭐야",
            "뭐예요",
            "뭔가요",
            "뭐지",
            "어떻게",
            "누가",
            "언제",
            "어디",
        ]
        .iter()
        .any(|marker| text.contains(marker))
        || text.starts_with("왜")
}

pub(super) fn match_search(view: &NormalizedUtterance<'_>) -> CommandMatch<SearchMatch> {
    let text = body(view);
    let mut search_endings = requests(&["검색해", "검색 해"]);
    search_endings.extend(["검색해", "검색해봐", "검색해 봐", "검색"].map(String::from));
    let mut find_endings = requests(&["찾아"]);
    find_endings.extend(["찾아봐", "찾아 봐"].map(String::from));

    // "찾아줘" alone is too common in ordinary questions; it searches only on a named site.
    let (end, needs_site) = if let Some(end) = ending_start(text, &search_endings) {
        (end, false)
    } else if let Some(end) = ending_start(text, &find_endings) {
        (end, true)
    } else {
        return CommandMatch::NoMatch;
    };
    let request = text[..end].trim_end();

    for (names, provider) in sites() {
        for name in names {
            let leading = format!("{name}에서 ");
            if let Some(query) = request.strip_prefix(&leading) {
                let start = request.len() - query.len();
                return query_match(view, start, request.len(), provider);
            }
            let trailing = format!(" {name}에서");
            if let Some(query) = request.strip_suffix(&trailing) {
                return query_match(view, 0, query.len(), provider);
            }
            if request == format!("{name}에서") {
                return CommandMatch::MissingPayload;
            }
        }
    }
    if needs_site {
        return CommandMatch::NoMatch;
    }
    if request.is_empty() {
        return CommandMatch::MissingPayload;
    }
    query_match(view, 0, request.len(), SearchProvider::Google)
}

fn query_match(
    view: &NormalizedUtterance<'_>,
    start: usize,
    end: usize,
    provider: SearchProvider,
) -> CommandMatch<SearchMatch> {
    view.original_for_match_range(start, end)
        .map(|query| CommandMatch::Matched(SearchMatch { provider, query }))
        .unwrap_or(CommandMatch::MissingPayload)
}

fn sites() -> [(&'static [&'static str], SearchProvider); 4] {
    [
        (&["구글", "google"], SearchProvider::Google),
        (&["유튜브", "youtube"], SearchProvider::YouTube),
        (&["아마존", "amazon"], SearchProvider::Amazon),
        (&["깃허브", "깃헙", "github"], SearchProvider::GitHub),
    ]
}
