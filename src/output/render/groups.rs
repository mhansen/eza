use ansiterm::Style;
use uzers::{Groups, Users};

use crate::fs::fields as f;
use crate::output::cell::TextCell;
use crate::output::table::{GroupFormat, UserFormat};

pub trait Render {
    fn render<C: Colours, U: Users + Groups>(
        self,
        colours: &C,
        users: &U,
        user_format: UserFormat,
        group_format: GroupFormat,
    ) -> TextCell;
}

impl Render for Option<f::Group> {
    fn render<C: Colours, U: Users + Groups>(
        self,
        colours: &C,
        users: &U,
        user_format: UserFormat,
        group_format: GroupFormat,
    ) -> TextCell {
        use uzers::os::unix::GroupExt;

        let mut style = colours.not_yours();

        let group = match self {
            Some(g) => match users.get_group_by_gid(g.0) {
                Some(g) => (*g).clone(),
                None => return TextCell::paint(style, g.0.to_string()),
            },
            None => return TextCell::blank(colours.no_group()),
        };

        let current_uid = users.get_current_uid();
        if let Some(current_user) = users.get_user_by_uid(current_uid) {
            if current_user.primary_group_id() == group.gid()
                || group.members().iter().any(|u| u == current_user.name())
            {
                style = colours.yours();
            }
        }

        if group.gid() == 0 && style != colours.yours() {
            style = colours.root_group();
        }

        let mut group_name = match user_format {
            UserFormat::Name => group.name().to_string_lossy().into(),
            UserFormat::Numeric => group.gid().to_string(),
        };

        group_name = match group_format {
            GroupFormat::Smart => {
                if let Some(current_user) = users.get_user_by_uid(current_uid) {
                    if current_user.name() == group.name() {
                        ":".to_string()
                    } else {
                        group_name
                    }
                } else {
                    group_name
                }
            }
            GroupFormat::Regular => group_name,
        };

        TextCell::paint(style, group_name)
    }
}

pub trait Colours {
    fn yours(&self) -> Style;
    fn not_yours(&self) -> Style;
    fn no_group(&self) -> Style;
    fn root_group(&self) -> Style;
}

#[cfg(test)]
#[allow(unused_results)]
pub mod test {
    use super::{Colours, Render};
    use crate::fs::fields as f;
    use crate::output::cell::TextCell;
    use crate::output::table::{GroupFormat, UserFormat};

    use ansiterm::Colour::*;
    use ansiterm::Style;
    use uzers::mock::MockUsers;
    use uzers::os::unix::GroupExt;
    use uzers::{Group, User};

    struct TestColours;

    #[rustfmt::skip]
    impl Colours for TestColours {
        fn yours(&self)     -> Style { Fixed(80).normal() }
        fn not_yours(&self) -> Style { Fixed(81).normal() }
        fn no_group(&self)   -> Style { Black.italic() }
        fn root_group(&self) -> Style { Fixed(82).normal() }
    }

    #[test]
    fn named() {
        let mut users = MockUsers::with_current_uid(1000);
        users.add_group(Group::new(100, "folk"));

        let group = Some(f::Group(100));
        let expected = TextCell::paint_str(Fixed(81).normal(), "folk");
        assert_eq!(
            expected,
            group.render(&TestColours, &users, UserFormat::Name, GroupFormat::Regular)
        );

        let expected = TextCell::paint_str(Fixed(81).normal(), "100");
        assert_eq!(
            expected,
            group.render(
                &TestColours,
                &users,
                UserFormat::Numeric,
                GroupFormat::Regular
            )
        );
    }

    #[test]
    fn unnamed() {
        let users = MockUsers::with_current_uid(1000);

        let group = Some(f::Group(100));
        let expected = TextCell::paint_str(Fixed(81).normal(), "100");
        assert_eq!(
            expected,
            group.render(&TestColours, &users, UserFormat::Name, GroupFormat::Regular)
        );
        assert_eq!(
            expected,
            group.render(
                &TestColours,
                &users,
                UserFormat::Numeric,
                GroupFormat::Regular
            )
        );
    }

    #[test]
    fn primary() {
        let mut users = MockUsers::with_current_uid(2);
        users.add_user(User::new(2, "eve", 100));
        users.add_group(Group::new(100, "folk"));

        let group = Some(f::Group(100));
        let expected = TextCell::paint_str(Fixed(80).normal(), "folk");
        assert_eq!(
            expected,
            group.render(&TestColours, &users, UserFormat::Name, GroupFormat::Regular)
        )
    }

    #[test]
    fn secondary() {
        let mut users = MockUsers::with_current_uid(2);
        users.add_user(User::new(2, "eve", 666));

        let test_group = Group::new(100, "folk").add_member("eve");
        users.add_group(test_group);

        let group = Some(f::Group(100));
        let expected = TextCell::paint_str(Fixed(80).normal(), "folk");
        assert_eq!(
            expected,
            group.render(&TestColours, &users, UserFormat::Name, GroupFormat::Regular)
        )
    }

    #[test]
    fn overflow() {
        let group = Some(f::Group(2_147_483_648));
        let expected = TextCell::paint_str(Fixed(81).normal(), "2147483648");
        assert_eq!(
            expected,
            group.render(
                &TestColours,
                &MockUsers::with_current_uid(0),
                UserFormat::Numeric,
                GroupFormat::Regular
            )
        );
    }

    #[test]
    fn smart() {
        let mut users = MockUsers::with_current_uid(1000);
        users.add_user(User::new(1000, "user", 110));
        users.add_group(Group::new(100, "user"));
        users.add_group(Group::new(101, "http"));

        let same_group = Some(f::Group(100));
        let expected = TextCell::paint_str(Fixed(81).normal(), ":");
        assert_eq!(
            expected,
            same_group.render(&TestColours, &users, UserFormat::Name, GroupFormat::Smart)
        );

        let expected = TextCell::paint_str(Fixed(81).normal(), ":");
        assert_eq!(
            expected,
            same_group.render(
                &TestColours,
                &users,
                UserFormat::Numeric,
                GroupFormat::Smart
            )
        );

        let http_group = Some(f::Group(101));
        let expected = TextCell::paint_str(Fixed(81).normal(), "http");
        assert_eq!(
            expected,
            http_group.render(&TestColours, &users, UserFormat::Name, GroupFormat::Smart)
        );
    }
}
