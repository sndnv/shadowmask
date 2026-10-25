use domain::common::PageRequest;
use domain::diagnostics::{PlayerId, Report, ReportId, ReportSummary};
use domain::repository::ReportRepository;
use domain::user::UserId;
use jiff::{SignedDuration, Timestamp};

fn at(secs: i64) -> Timestamp {
    Timestamp::from_second(secs).expect("valid timestamp")
}

fn hours_before(now: Timestamp, hours: i64) -> Timestamp {
    now.checked_sub(SignedDuration::from_hours(hours)).expect("valid timestamp")
}

fn report(id: &str, created_at: Timestamp, note: Option<&str>) -> Report {
    Report {
        summary: ReportSummary {
            id: ReportId(id.into()),
            created_at,
            user: UserId("u1".into()),
            player: PlayerId(format!("player-{id}")),
            app: "flutter".into(),
            platform: "Android 14".into(),
            app_version: "0.0.7".into(),
            note: note.map(str::to_owned),
        },
        payload: format!("{{\"schema\":1,\"id\":\"{id}\"}}"),
    }
}

fn ids(items: &[ReportSummary]) -> Vec<&str> {
    items.iter().map(|r| r.id.0.as_str()).collect()
}

pub async fn report_repository_contract<R: ReportRepository>(repo: R) {
    let now = at(1_700_000_000);
    let window = hours_before(now, 24);
    let missing = ReportId("missing".into());

    assert_eq!(repo.list(window, None, PageRequest::ALL).await.unwrap().total, 0);
    assert!(repo.get(&missing, window).await.unwrap().is_none());
    assert!(!repo.delete(&missing).await.unwrap());
    assert_eq!(repo.delete_before(window).await.unwrap(), 0);

    let newest = report("newest", hours_before(now, 1), Some("stutters after a seek"));
    repo.insert(newest.clone()).await.unwrap();
    repo.insert(report("tie-b", hours_before(now, 2), None)).await.unwrap();
    repo.insert(report("tie-a", hours_before(now, 2), None)).await.unwrap();
    repo.insert(report("expired", hours_before(now, 30), None)).await.unwrap();

    let listed = repo.list(window, None, PageRequest::ALL).await.unwrap();
    assert_eq!(ids(&listed.items), ["newest", "tie-a", "tie-b"], "newest first, then by id");
    assert_eq!(listed.total, 3, "a report past the window is left out of the count too");

    let second = repo.list(window, None, PageRequest { offset: 1, limit: 1 }).await.unwrap();
    assert_eq!(ids(&second.items), ["tie-a"]);
    assert_eq!((second.total, second.offset, second.limit), (3, 1, 1));

    assert_eq!(repo.get(&newest.summary.id, window).await.unwrap(), Some(newest.clone()));
    assert_eq!(listed.items[0], newest.summary, "the list carries every field but the payload");
    let expired = ReportId("expired".into());
    assert!(repo.get(&expired, window).await.unwrap().is_none(), "past the window reads as gone");
    assert!(repo.get(&expired, hours_before(now, 48)).await.unwrap().is_some());

    assert!(repo.delete(&ReportId("tie-b".into())).await.unwrap());
    assert!(!repo.delete(&ReportId("tie-b".into())).await.unwrap(), "a second delete finds none");
    let kept = repo.list(window, None, PageRequest::ALL).await.unwrap();
    assert_eq!(ids(&kept.items), ["newest", "tie-a"]);

    assert_eq!(repo.delete_before(window).await.unwrap(), 1);
    assert!(repo.get(&expired, hours_before(now, 48)).await.unwrap().is_none());
    assert_eq!(repo.list(hours_before(now, 48), None, PageRequest::ALL).await.unwrap().total, 2);

    let mut roku = report("roku-1", hours_before(now, 3), Some("Android was fine"));
    roku.summary.player = PlayerId("device-9".into());
    roku.summary.app = "roku".into();
    roku.summary.platform = "Roku Ultra (4800X), Roku OS 13.0.4".into();
    repo.insert(roku).await.unwrap();
    let found = |needle: &'static str| repo.list(window, Some(needle), PageRequest::ALL);
    assert_eq!(ids(&found("ROKU").await.unwrap().items), ["roku-1"], "any case");
    assert_eq!(ids(&found("ultra (48").await.unwrap().items), ["roku-1"], "a part of the platform");
    assert_eq!(ids(&found("device-9").await.unwrap().items), ["roku-1"], "the player");
    assert_eq!(ids(&found("android").await.unwrap().items), ["newest", "tie-a"], "not the note");
    assert!(found("stutters").await.unwrap().items.is_empty(), "the note is not searched");
    assert!(found("%").await.unwrap().items.is_empty(), "a wildcard matches itself");
    assert!(found("player_").await.unwrap().items.is_empty(), "so does an underscore");
    assert_eq!(found("u1").await.unwrap().total, 3, "the user");
    let paged = repo.list(window, Some("player-"), PageRequest { offset: 1, limit: 1 }).await;
    let paged = paged.unwrap();
    assert_eq!(ids(&paged.items), ["tie-a"]);
    assert_eq!(paged.total, 2, "the total counts the matches, before the page");
}
