use crate::matcher::RouteInfo;
use crate::render::report_model::DiffReport;
use crate::render::DiffReportRenderer;
use crate::rules::MatchResult;
use std::error::Error;
use tera::{Context, Tera};

pub struct HtmlRenderer {
    tera: Tera,
}

impl HtmlRenderer {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        // Load templates from the templates directory
        let mut tera = Tera::default();

        // Load main template
        let _ = tera.add_raw_template("report.html", include_str!("../../templates/report.html"));

        // Load component templates
        let _ = tera.add_raw_template(
            "components/base_styles.html",
            include_str!("../../templates/components/base_styles.html"),
        );
        let _ = tera.add_raw_template(
            "components/header.html",
            include_str!("../../templates/components/header.html"),
        );
        let _ = tera.add_raw_template(
            "components/stats.html",
            include_str!("../../templates/components/stats.html"),
        );
        let _ = tera.add_raw_template(
            "components/help.html",
            include_str!("../../templates/components/help.html"),
        );
        let _ = tera.add_raw_template(
            "components/grouped_changes.html",
            include_str!("../../templates/components/grouped_changes.html"),
        );
        let _ = tera.add_raw_template(
            "components/routes.html",
            include_str!("../../templates/components/routes.html"),
        );
        let _ = tera.add_raw_template(
            "components/schemas.html",
            include_str!("../../templates/components/schemas.html"),
        );
        let _ = tera.add_raw_template(
            "components/scripts.html",
            include_str!("../../templates/components/scripts.html"),
        );

        Ok(Self { tera })
    }

    /// Render HTML report with routes and schemas
    pub fn render_with_routes(
        &self,
        schema_results: &[MatchResult],
        route_results: &[MatchResult],
        route_infos: &[RouteInfo],
        full_schema_infos: &[crate::rules::FullSchemaInfo],
    ) -> Result<String, Box<dyn Error>> {
        let report = DiffReport::from_match_results(
            schema_results,
            route_results,
            route_infos,
            full_schema_infos,
        );
        self.render_report(&report)
    }
}

impl DiffReportRenderer for HtmlRenderer {
    fn render_report(&self, report: &DiffReport) -> Result<String, Box<dyn Error>> {
        let mut context = Context::new();
        context.insert("data", report);

        let html = self.tera.render("report.html", &context)?;
        Ok(html)
    }

    fn file_extension(&self) -> &'static str {
        "html"
    }
}
