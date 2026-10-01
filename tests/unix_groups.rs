#![cfg(target_os = "linux")]
#[test]
fn user_group_membership_matches_the_operating_system_without_invented_root_group() {
    let account = users::get_user_by_name("nobody").expect("Linux test image must provide nobody");
    let result = std::process::Command::new("id").args(["-G", "nobody"]).output().unwrap();
    assert!(result.status.success());
    let mut expected = String::from_utf8(result.stdout).unwrap().split_whitespace().map(|gid|gid.parse::<u32>().unwrap()).collect::<Vec<_>>();
    expected.sort_unstable(); expected.dedup();
    let mut actual = users::get_user_groups(account.name(), account.primary_group_id()).unwrap().iter().map(|group|group.gid()).collect::<Vec<_>>();
    actual.sort_unstable(); actual.dedup();
    assert_eq!(actual, expected, "group query must not invent root membership");
    let current = users::get_current_uid();
    let user = users::get_user_by_uid(current).unwrap();
    assert_eq!(user.uid(), current);
}
