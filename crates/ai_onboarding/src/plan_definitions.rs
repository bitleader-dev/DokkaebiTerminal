use gpui::{App, IntoElement, ParentElement, SharedString};
use i18n::t;
use ui::{List, ListBulletItem, prelude::*};

/// AI 플랜 관련 공통 정의
pub struct PlanDefinitions;

impl PlanDefinitions {
    pub fn ai_description(cx: &App) -> SharedString {
        t("ai_onboarding.ai_description", cx)
    }

    pub fn free_plan(&self) -> impl IntoElement {
        List::new()
            .child(ListBulletItem::new("2,000 accepted edit predictions"))
            .child(ListBulletItem::new(
                "Unlimited prompts with your AI API keys",
            ))
            .child(ListBulletItem::new(
                "Unlimited use of external agents like Claude Agent",
            ))
    }

    pub fn pro_trial(&self, period: bool) -> impl IntoElement {
        List::new()
            .child(ListBulletItem::new("Unlimited edit predictions"))
            .child(ListBulletItem::new("$20 of tokens"))
            .when(period, |this| {
                this.child(ListBulletItem::new(
                    "Try it out for 14 days, no credit card required",
                ))
            })
    }

    pub fn pro_plan(&self) -> impl IntoElement {
        List::new()
            .child(ListBulletItem::new("Unlimited edit predictions"))
            .child(ListBulletItem::new("$5 of tokens"))
            .child(ListBulletItem::new("Usage-based billing beyond $5"))
    }

    pub fn business_plan(&self) -> impl IntoElement {
        List::new()
            .child(ListBulletItem::new("Unlimited edit predictions"))
            .child(ListBulletItem::new("Usage-based billing"))
    }

    pub fn student_plan(&self) -> impl IntoElement {
        List::new()
            .child(ListBulletItem::new("Unlimited edit predictions"))
            .child(ListBulletItem::new("$10 of tokens"))
            .child(ListBulletItem::new(
                "Optional credit packs for additional usage",
            ))
    }
}
