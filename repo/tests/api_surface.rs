//! One real HTTP request per configured API route (smoke / routing coverage).
//! Uses `CIVICSORT_API_URL` — real TCP only (see `route_catalog` for the full route list).

use crate::common;
use crate::route_catalog;
use uuid::Uuid;

const DUMMY: &str = "00000000-0000-0000-0000-000000000001";

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

async fn surface_route_hit(counter: &mut u32, label: &str, code: u16, body: &str) {
    common::maybe_throttle_rate_limit(counter).await;
    common::assert_api_surface_status(label, code, body);
}

#[tokio::test]
async fn every_api_route_handles_request_over_http() {
    let mut n = 0u32;

    let (ch, bh) = api_get!("/api/health", None).await;
    surface_route_hit(&mut n, "GET /api/health", ch, &bh).await;

    let username = common::default_itest_username();
    let password = common::default_itest_password();
    let reg = serde_json::json!({
        "username": username,
        "password": password,
        "role": "OperationsAdmin"
    });
    let (cr, br) = api_post!("/api/auth/register", &reg.to_string(), None).await;
    surface_route_hit(&mut n, "POST /api/auth/register", cr, &br).await;

    let login_body = serde_json::json!({ "username": username, "password": password });
    let (cl, bl) = api_post!("/api/auth/login", &login_body.to_string(), None).await;
    surface_route_hit(&mut n, "POST /api/auth/login", cl, &bl).await;
    assert_eq!(cl, 200, "POST /api/auth/login: {}", bl);
    let (token, user_id) = common::parse_session_from_login_body(&bl);
    let t = Some(token.as_str());

    let (c_sess, b_sess) = api_get!("/api/auth/session", t).await;
    surface_route_hit(&mut n, "GET /api/auth/session", c_sess, &b_sess).await;

    let stepup = serde_json::json!({ "password": password, "action_type": "export_csv" }).to_string();
    let (c_su, b_su) = api_post!("/api/auth/stepup", &stepup, t).await;
    surface_route_hit(&mut n, "POST /api/auth/stepup", c_su, &b_su).await;
    if c_su == 200 {
        common::assert_field("POST /api/auth/stepup", &b_su, "message");
        common::assert_field("POST /api/auth/stepup", &b_su, "action_type");
        common::assert_field("POST /api/auth/stepup", &b_su, "expires_at");
    }

    let (c_users, b_users) = api_get!("/api/users", t).await;
    surface_route_hit(&mut n, "GET /api/users", c_users, &b_users).await;
    if c_users == 200 {
        common::assert_is_array("GET /api/users", &b_users);
    }
    let (cu, bu) = api_get!(&format!("/api/users/{}", user_id), t).await;
    surface_route_hit(&mut n, "GET /api/users/{id}", cu, &bu).await;

    let role_body = serde_json::json!({ "role": "OperationsAdmin" }).to_string();
    let (c_role, b_role) = api_put!(&format!("/api/users/{}/role", user_id), &role_body, t).await;
    surface_route_hit(&mut n, "PUT /api/users/{id}/role", c_role, &b_role).await;

    let (c_dev_list, b_dev_list) = api_get!("/api/devices", t).await;
    surface_route_hit(&mut n, "GET /api/devices", c_dev_list, &b_dev_list).await;
    let bind = serde_json::json!({
        "device_fingerprint": "integration-test-fp-surface",
        "device_name": "itest"
    })
    .to_string();
    let (cb, bb) = api_post!("/api/devices/bind", &bind, t).await;
    surface_route_hit(&mut n, "POST /api/devices/bind", cb, &bb).await;

    let trust = serde_json::json!({ "device_id": DUMMY, "password": password }).to_string();
    let (ct, bt) = api_post!("/api/devices/trust", &trust, t).await;
    surface_route_hit(&mut n, "POST /api/devices/trust", ct, &bt).await;

    let (cd, bd) = api_delete!(&format!("/api/devices/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "DELETE /api/devices/{id}", cd, &bd).await;

    let (c_au, b_au) = api_get!("/api/audit?page=1&page_size=5", t).await;
    surface_route_hit(&mut n, "GET /api/audit", c_au, &b_au).await;
    let (cae, bae) = api_get!("/api/audit/export?format=csv", t).await;
    surface_route_hit(&mut n, "GET /api/audit/export", cae, &bae).await;
    let (cai, bai) = api_get!("/api/audit/integrity", t).await;
    surface_route_hit(&mut n, "GET /api/audit/integrity", cai, &bai).await;

    let (c_dash, b_dash) = api_get!("/api/admin/dashboard", t).await;
    surface_route_hit(&mut n, "GET /api/admin/dashboard", c_dash, &b_dash).await;
    if c_dash == 200 {
        common::assert_field("GET /api/admin/dashboard", &b_dash, "active_users");
    }
    let (c_kpi, b_kpi) = api_get!("/api/admin/kpi/trend", t).await;
    surface_route_hit(&mut n, "GET /api/admin/kpi/trend", c_kpi, &b_kpi).await;
    let (c_ou, b_ou) = api_get!("/api/admin/overview/users", t).await;
    surface_route_hit(&mut n, "GET /api/admin/overview/users", c_ou, &b_ou).await;
    let (c_oi, b_oi) = api_get!("/api/admin/overview/items", t).await;
    surface_route_hit(&mut n, "GET /api/admin/overview/items", c_oi, &b_oi).await;
    let (c_ow, b_ow) = api_get!("/api/admin/overview/workorders", t).await;
    surface_route_hit(&mut n, "GET /api/admin/overview/workorders", c_ow, &b_ow).await;

    let camp = serde_json::json!({
        "name": "itest campaign",
        "start_date": "2026-01-01",
        "end_date": "2026-12-31"
    })
    .to_string();
    let (ccp, bcp) = api_post!("/api/admin/campaigns", &camp, t).await;
    surface_route_hit(&mut n, "POST /api/admin/campaigns", ccp, &bcp).await;
    if ccp == 201 {
        common::assert_field("POST /api/admin/campaigns", &bcp, "campaign.id");
    }
    let (c_camps, b_camps) = api_get!("/api/admin/campaigns", t).await;
    surface_route_hit(&mut n, "GET /api/admin/campaigns", c_camps, &b_camps).await;
    let (cg, bg) = api_get!(&format!("/api/admin/campaigns/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "GET /api/admin/campaigns/{id}", cg, &bg).await;
    let cupd = serde_json::json!({ "name": "x" }).to_string();
    let (ccu, bcu) = api_put!(&format!("/api/admin/campaigns/{}", DUMMY), &cupd, t).await;
    surface_route_hit(&mut n, "PUT /api/admin/campaigns/{id}", ccu, &bcu).await;

    let tag = serde_json::json!({ "name": "itest-tag-surface" }).to_string();
    let (cta, bta) = api_post!("/api/admin/tags", &tag, t).await;
    surface_route_hit(&mut n, "POST /api/admin/tags", cta, &bta).await;
    if cta == 201 {
        common::assert_field("POST /api/admin/tags", &bta, "id");
        common::assert_field("POST /api/admin/tags", &bta, "name");
    }
    let (c_tags, b_tags) = api_get!("/api/admin/tags", t).await;
    surface_route_hit(&mut n, "GET /api/admin/tags", c_tags, &b_tags).await;
    if c_tags == 200 {
        common::assert_is_array("GET /api/admin/tags", &b_tags);
    }
    let (ctd, btd) = api_delete!(&format!("/api/admin/tags/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "DELETE /api/admin/tags/{id}", ctd, &btd).await;
    let (ctg, btg) = api_put!(&format!("/api/admin/categories/{}/tags", DUMMY), "[]", t).await;
    surface_route_hit(&mut n, "PUT /api/admin/categories/{id}/tags", ctg, &btg).await;

    let gen = serde_json::json!({
        "report_type": "kpi_summary",
        "format": "Csv"
    })
    .to_string();
    let (cgn, bgn) = api_post!("/api/admin/reports/generate", &gen, t).await;
    surface_route_hit(&mut n, "POST /api/admin/reports/generate", cgn, &bgn).await;
    let rcfg = serde_json::json!({
        "name": "cfg",
        "report_type": "kpi_summary"
    })
    .to_string();
    let (crc, brc) = api_post!("/api/admin/reports/configs", &rcfg, t).await;
    surface_route_hit(&mut n, "POST /api/admin/reports/configs", crc, &brc).await;
    let (c_cfgs, b_cfgs) = api_get!("/api/admin/reports/configs", t).await;
    surface_route_hit(&mut n, "GET /api/admin/reports/configs", c_cfgs, &b_cfgs).await;

    let imp = serde_json::json!({
        "name": "itest",
        "entity_type": "kb_entry",
        "rows": [serde_json::json!({"item_name": "x"})]
    })
    .to_string();
    let (cim, bim) = api_post!("/api/bulk/import", &imp, t).await;
    surface_route_hit(&mut n, "POST /api/bulk/import", cim, &bim).await;
    if cim == 201 {
        common::assert_field("POST /api/bulk/import", &bim, "job.id");
    }
    let (c_imps, b_imps) = api_get!("/api/bulk/import", t).await;
    surface_route_hit(&mut n, "GET /api/bulk/import", c_imps, &b_imps).await;
    let (cij, bij) = api_get!(&format!("/api/bulk/import/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "GET /api/bulk/import/{id}", cij, &bij).await;
    if cij >= 400 {
        common::assert_error_field("GET /api/bulk/import/{id}", &bij);
    }
    let (cex, bex) = api_post!(&format!("/api/bulk/import/{}/execute", DUMMY), "{}", t).await;
    surface_route_hit(&mut n, "POST /api/bulk/import/{id}/execute", cex, &bex).await;

    let exp = serde_json::json!({ "entity_type": "kb_entry", "format": "csv" }).to_string();
    let (cbe, bbe) = api_post!("/api/bulk/export", &exp, t).await;
    surface_route_hit(&mut n, "POST /api/bulk/export", cbe, &bbe).await;
    let (c_ch, b_ch) = api_get!("/api/bulk/changes", t).await;
    surface_route_hit(&mut n, "GET /api/bulk/changes", c_ch, &b_ch).await;
    let (cbr, bbr) = api_post!(&format!("/api/bulk/changes/{}/revert", DUMMY), "{}", t).await;
    surface_route_hit(&mut n, "POST /api/bulk/changes/{id}/revert", cbr, &bbr).await;
    let (c_dup, b_dup) = api_get!("/api/bulk/duplicates", t).await;
    surface_route_hit(&mut n, "GET /api/bulk/duplicates", c_dup, &b_dup).await;
    let (crd, brd) = api_put!(
        &format!("/api/bulk/duplicates/{}/resolve", DUMMY),
        &serde_json::json!({ "status": "Dismissed" }).to_string(),
        t,
    )
    .await;
    surface_route_hit(&mut n, "PUT /api/bulk/duplicates/{id}/resolve", crd, &brd).await;

    let merge = serde_json::json!({
        "entity_type": "kb_entry",
        "source_id": DUMMY,
        "target_id": DUMMY
    })
    .to_string();
    let (cmg, bmg) = api_post!("/api/bulk/merges", &merge, t).await;
    surface_route_hit(&mut n, "POST /api/bulk/merges", cmg, &bmg).await;
    if cmg == 201 {
        common::assert_field("POST /api/bulk/merges", &bmg, "request.id");
    }
    let (c_mgs, b_mgs) = api_get!("/api/bulk/merges", t).await;
    surface_route_hit(&mut n, "GET /api/bulk/merges", c_mgs, &b_mgs).await;
    let (cm1, bm1) = api_get!(&format!("/api/bulk/merges/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "GET /api/bulk/merges/{id}", cm1, &bm1).await;
    if cm1 >= 400 {
        common::assert_error_field("GET /api/bulk/merges/{id}", &bm1);
    }
    let (cm2, bm2) = api_put!(
        &format!("/api/bulk/merges/{}/conflicts/{}", DUMMY, DUMMY),
        &serde_json::json!({ "resolution": "keep_target" }).to_string(),
        t,
    )
    .await;
    surface_route_hit(&mut n, "PUT /api/bulk/merges/{id}/conflicts/{cid}", cm2, &bm2).await;
    let (cm3, bm3) = api_put!(
        &format!("/api/bulk/merges/{}/review", DUMMY),
        &serde_json::json!({ "status": "Approved" }).to_string(),
        t,
    )
    .await;
    surface_route_hit(&mut n, "PUT /api/bulk/merges/{id}/review", cm3, &bm3).await;

    let kb_entry = serde_json::json!({
        "item_name": "itest kb surface",
        "disposal_category": "cat",
        "disposal_instructions": "instr"
    })
    .to_string();
    let (ckb, bkb) = api_post!("/api/kb/entries", &kb_entry, t).await;
    surface_route_hit(&mut n, "POST /api/kb/entries", ckb, &bkb).await;
    let kb_id = if ckb == 201 {
        common::assert_field("POST /api/kb/entries", &bkb, "entry.id");
        common::extract_uuid("POST /api/kb/entries", &bkb, "entry.id")
    } else {
        Uuid::nil()
    };

    let (c_ks, b_ks) = api_get!("/api/kb/search?q=test&region=default", t).await;
    surface_route_hit(&mut n, "GET /api/kb/search", c_ks, &b_ks).await;
    let (ckg, bkg) = api_get!(&format!("/api/kb/entries/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "GET /api/kb/entries/{id}", ckg, &bkg).await;
    if ckg >= 400 {
        common::assert_error_field("GET /api/kb/entries/{id}", &bkg);
    }
    let upd_kb = serde_json::json!({
        "disposal_category": "cat",
        "disposal_instructions": "i2"
    })
    .to_string();
    let (cku, bku) = api_put!(&format!("/api/kb/entries/{}", DUMMY), &upd_kb, t).await;
    surface_route_hit(&mut n, "PUT /api/kb/entries/{id}", cku, &bku).await;
    let (ckd, bkd) = api_delete!(&format!("/api/kb/entries/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "DELETE /api/kb/entries/{id}", ckd, &bkd).await;
    let (ckv, bkv) = api_get!(&format!("/api/kb/entries/{}/versions", DUMMY), t).await;
    surface_route_hit(&mut n, "GET /api/kb/entries/{id}/versions", ckv, &bkv).await;
    let (c_kc, b_kc) = api_get!("/api/kb/categories", t).await;
    surface_route_hit(&mut n, "GET /api/kb/categories", c_kc, &b_kc).await;
    if c_kc == 200 {
        common::assert_is_array("GET /api/kb/categories", &b_kc);
    }
    let cat = serde_json::json!({ "name": "itest-cat" }).to_string();
    let (cca, bca) = api_post!("/api/kb/categories", &cat, t).await;
    surface_route_hit(&mut n, "POST /api/kb/categories", cca, &bca).await;

    let (cpi, bpi) = api_post_bytes!("/api/kb/images", "image/png", PNG_1X1, t).await;
    surface_route_hit(&mut n, "POST /api/kb/images", cpi, &bpi).await;
    let (cig, big) = api_get!(&format!("/api/kb/images/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "GET /api/kb/images/{id}", cig, &big).await;
    let (c_sk, b_sk) = api_get!("/api/kb/search-config", t).await;
    surface_route_hit(&mut n, "GET /api/kb/search-config", c_sk, &b_sk).await;
    let scfg = serde_json::json!({
        "name_exact_weight": 1.0,
        "name_prefix_weight": 0.5,
        "name_fuzzy_weight": 0.3,
        "alias_exact_weight": 0.8,
        "alias_fuzzy_weight": 0.2,
        "category_boost": 0.1,
        "region_boost": 0.1,
        "recency_boost": 0.05,
        "fuzzy_threshold": 0.6,
        "max_results": 20
    })
    .to_string();
    let (csu, bsu) = api_put!("/api/kb/search-config", &scfg, t).await;
    surface_route_hit(&mut n, "PUT /api/kb/search-config", csu, &bsu).await;

    let dis = serde_json::json!({
        "kb_entry_id": if kb_id != Uuid::nil() { kb_id } else { Uuid::parse_str(DUMMY).unwrap() },
        "reason": "integration surface"
    })
    .to_string();
    let (cds, bds) = api_post!("/api/disputes", &dis, t).await;
    surface_route_hit(&mut n, "POST /api/disputes", cds, &bds).await;
    let dispute_id = if cds == 201 {
        common::assert_field("POST /api/disputes", &bds, "id");
        common::extract_uuid("POST /api/disputes", &bds, "id")
    } else {
        Uuid::nil()
    };

    let (c_dss, b_dss) = api_get!("/api/disputes", t).await;
    surface_route_hit(&mut n, "GET /api/disputes", c_dss, &b_dss).await;
    if c_dss == 200 {
        common::assert_is_array("GET /api/disputes", &b_dss);
    }
    let did = if dispute_id != Uuid::nil() {
        dispute_id.to_string()
    } else {
        DUMMY.to_string()
    };
    let (cdg, bdg) = api_get!(&format!("/api/disputes/{}", did), t).await;
    surface_route_hit(&mut n, "GET /api/disputes/{id}", cdg, &bdg).await;
    let res = serde_json::json!({ "status": "Dismissed", "resolution_notes": null }).to_string();
    let (cdr, bdr) = api_put!(&format!("/api/disputes/{}/resolve", did), &res, t).await;
    surface_route_hit(&mut n, "PUT /api/disputes/{id}/resolve", cdr, &bdr).await;

    let tmpl = serde_json::json!({
        "name": "itest template surface",
        "cycle": "Daily"
    })
    .to_string();
    let (cit, bit) = api_post!("/api/inspection/templates", &tmpl, t).await;
    surface_route_hit(&mut n, "POST /api/inspection/templates", cit, &bit).await;
    let template_id = if cit == 201 {
        common::assert_field("POST /api/inspection/templates", &bit, "template.id");
        common::extract_uuid("POST /api/inspection/templates", &bit, "template.id")
    } else {
        Uuid::nil()
    };

    let (c_tl, b_tl) = api_get!("/api/inspection/templates", t).await;
    surface_route_hit(&mut n, "GET /api/inspection/templates", c_tl, &b_tl).await;
    if c_tl == 200 {
        common::assert_is_array("GET /api/inspection/templates", &b_tl);
    }
    let tid = if template_id != Uuid::nil() {
        template_id.to_string()
    } else {
        DUMMY.to_string()
    };
    let (ctg2, btg2) = api_get!(&format!("/api/inspection/templates/{}", tid), t).await;
    surface_route_hit(&mut n, "GET /api/inspection/templates/{id}", ctg2, &btg2).await;
    let ut = serde_json::json!({ "name": "renamed" }).to_string();
    let (cut, but) = api_put!(&format!("/api/inspection/templates/{}", tid), &ut, t).await;
    surface_route_hit(&mut n, "PUT /api/inspection/templates/{id}", cut, &but).await;
    let (cdt, bdt) = api_delete!(&format!("/api/inspection/templates/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "DELETE /api/inspection/templates/{id}", cdt, &bdt).await;
    let st = serde_json::json!([]).to_string();
    let (cst, bst) = api_put!(&format!("/api/inspection/templates/{}/subtasks", tid), &st, t).await;
    surface_route_hit(&mut n, "PUT /api/inspection/templates/{id}/subtasks", cst, &bst).await;

    let sched = serde_json::json!({
        "template_id": if template_id != Uuid::nil() { template_id } else { Uuid::parse_str(DUMMY).unwrap() },
        "assigned_to": user_id,
        "start_date": "2026-04-01"
    })
    .to_string();
    let (csc, bsc) = api_post!("/api/inspection/schedules", &sched, t).await;
    surface_route_hit(&mut n, "POST /api/inspection/schedules", csc, &bsc).await;
    if csc == 201 {
        common::assert_field("POST /api/inspection/schedules", &bsc, "schedule.id");
        common::assert_field("POST /api/inspection/schedules", &bsc, "instances_generated");
    }
    let (c_sch, b_sch) = api_get!("/api/inspection/schedules", t).await;
    surface_route_hit(&mut n, "GET /api/inspection/schedules", c_sch, &b_sch).await;
    if c_sch == 200 {
        common::assert_is_array("GET /api/inspection/schedules", &b_sch);
    }
    let (c_tsk, b_tsk) = api_get!("/api/inspection/tasks", t).await;
    surface_route_hit(&mut n, "GET /api/inspection/tasks", c_tsk, &b_tsk).await;
    if c_tsk == 200 {
        common::assert_field("GET /api/inspection/tasks", &b_tsk, "tasks");
    }
    let (ctk, btk) = api_get!(&format!("/api/inspection/tasks/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "GET /api/inspection/tasks/{id}", ctk, &btk).await;
    let (cst2, bst2) = api_post!(&format!("/api/inspection/tasks/{}/start", DUMMY), "{}", t).await;
    surface_route_hit(&mut n, "POST /api/inspection/tasks/{id}/start", cst2, &bst2).await;

    let sub = serde_json::json!({
        "instance_id": DUMMY,
        "responses": []
    })
    .to_string();
    let (csu2, bsu2) = api_post!("/api/inspection/submissions", &sub, t).await;
    surface_route_hit(&mut n, "POST /api/inspection/submissions", csu2, &bsu2).await;
    if csu2 == 201 {
        common::assert_field("POST /api/inspection/submissions", &bsu2, "submission.id");
        common::assert_field("POST /api/inspection/submissions", &bsu2, "valid");
    }
    let (csg, bsg) = api_get!(&format!("/api/inspection/submissions/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "GET /api/inspection/submissions/{id}", csg, &bsg).await;
    let revs = serde_json::json!({ "status": "Approved", "review_notes": null }).to_string();
    let (csr, bsr) = api_put!(&format!("/api/inspection/submissions/{}/review", DUMMY), &revs, t).await;
    surface_route_hit(&mut n, "PUT /api/inspection/submissions/{id}/review", csr, &bsr).await;

    let (c_rm, b_rm) = api_get!("/api/inspection/reminders", t).await;
    surface_route_hit(&mut n, "GET /api/inspection/reminders", c_rm, &b_rm).await;
    let (cmr, bmr) = api_post!("/api/inspection/reminders/read-all", "{}", t).await;
    surface_route_hit(&mut n, "POST /api/inspection/reminders/read-all", cmr, &bmr).await;
    let (c_rm_r, b_rm_r) =
        api_post!(&format!("/api/inspection/reminders/{}/read", DUMMY), "{}", t).await;
    surface_route_hit(&mut n, "POST /api/inspection/reminders/{id}/read", c_rm_r, &b_rm_r).await;
    let (c_rm_d, b_rm_d) =
        api_post!(&format!("/api/inspection/reminders/{}/dismiss", DUMMY), "{}", t).await;
    surface_route_hit(&mut n, "POST /api/inspection/reminders/{id}/dismiss", c_rm_d, &b_rm_d).await;

    let (cgi, bgi) = api_post!("/api/inspection/generate-instances", "{}", t).await;
    surface_route_hit(&mut n, "POST /api/inspection/generate-instances", cgi, &bgi).await;
    let (cpo, bpo) = api_post!("/api/inspection/process-overdue", "{}", t).await;
    surface_route_hit(&mut n, "POST /api/inspection/process-overdue", cpo, &bpo).await;

    let sc = serde_json::json!({
        "name": "sc",
        "target_type": "DisputedClassification",
        "passing_score": 0.7
    })
    .to_string();
    let (csc2, bsc2) = api_post!("/api/reviews/scorecards", &sc, t).await;
    surface_route_hit(&mut n, "POST /api/reviews/scorecards", csc2, &bsc2).await;
    let scorecard_id = if csc2 == 201 {
        common::assert_field("POST /api/reviews/scorecards", &bsc2, "scorecard.id");
        common::extract_uuid("POST /api/reviews/scorecards", &bsc2, "scorecard.id")
    } else {
        Uuid::nil()
    };

    let (c_scl, b_scl) = api_get!("/api/reviews/scorecards", t).await;
    surface_route_hit(&mut n, "GET /api/reviews/scorecards", c_scl, &b_scl).await;
    if c_scl == 200 {
        common::assert_is_array("GET /api/reviews/scorecards", &b_scl);
    }
    let sid = if scorecard_id != Uuid::nil() {
        scorecard_id.to_string()
    } else {
        DUMMY.to_string()
    };
    let (csg2, bsg2) = api_get!(&format!("/api/reviews/scorecards/{}", sid), t).await;
    surface_route_hit(&mut n, "GET /api/reviews/scorecards/{id}", csg2, &bsg2).await;
    let (csd, bsd) = api_put!(&format!("/api/reviews/scorecards/{}/dimensions", sid), "[]", t).await;
    surface_route_hit(&mut n, "PUT /api/reviews/scorecards/{id}/dimensions", csd, &bsd).await;

    let assign_tid = if dispute_id != Uuid::nil() {
        dispute_id
    } else {
        Uuid::parse_str(DUMMY).unwrap()
    };
    let assign_sc = if scorecard_id != Uuid::nil() {
        scorecard_id
    } else {
        Uuid::parse_str(DUMMY).unwrap()
    };
    let asn = serde_json::json!({
        "reviewer_id": null,
        "target_type": "DisputedClassification",
        "target_id": assign_tid,
        "scorecard_id": assign_sc
    })
    .to_string();
    let (cas, bas) = api_post!("/api/reviews/assignments", &asn, t).await;
    surface_route_hit(&mut n, "POST /api/reviews/assignments", cas, &bas).await;
    if cas == 201 {
        common::assert_field("POST /api/reviews/assignments", &bas, "id");
        common::assert_field("POST /api/reviews/assignments", &bas, "status");
    }

    let (c_rq, b_rq) = api_get!("/api/reviews/queue", t).await;
    surface_route_hit(&mut n, "GET /api/reviews/queue", c_rq, &b_rq).await;
    if c_rq == 200 {
        common::assert_field("GET /api/reviews/queue", &b_rq, "assignments");
    }
    let (cag, bag) = api_get!(&format!("/api/reviews/assignments/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "GET /api/reviews/assignments/{id}", cag, &bag).await;
    let rec = serde_json::json!({ "reason": "integration" }).to_string();
    let (car, bar) = api_post!(&format!("/api/reviews/assignments/{}/recuse", DUMMY), &rec, t).await;
    surface_route_hit(&mut n, "POST /api/reviews/assignments/{id}/recuse", car, &bar).await;
    let subm = serde_json::json!({
        "scores": [],
        "recommendation": "approve"
    })
    .to_string();
    let (cas2, bas2) = api_post!(&format!("/api/reviews/assignments/{}/submit", DUMMY), &subm, t).await;
    surface_route_hit(&mut n, "POST /api/reviews/assignments/{id}/submit", cas2, &bas2).await;

    let coi = serde_json::json!({
        "conflict_type": "department",
        "department": "d1"
    })
    .to_string();
    let (cco, bco) = api_post!("/api/reviews/coi", &coi, t).await;
    surface_route_hit(&mut n, "POST /api/reviews/coi", cco, &bco).await;
    if cco == 201 {
        common::assert_field("POST /api/reviews/coi", &bco, "id");
    }
    let (c_cl, b_cl) = api_get!("/api/reviews/coi", t).await;
    surface_route_hit(&mut n, "GET /api/reviews/coi", c_cl, &b_cl).await;
    if c_cl == 200 {
        common::assert_is_array("GET /api/reviews/coi", &b_cl);
    }
    let (ccd, bcd) = api_delete!(&format!("/api/reviews/coi/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "DELETE /api/reviews/coi/{id}", ccd, &bcd).await;
    let (crw, brw) = api_get!(&format!("/api/reviews/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "GET /api/reviews/{id}", crw, &brw).await;

    // When "tmpl" already exists in the DB (409), fetch the real ID from the list so the
    // trigger creation gets a valid FK reference instead of Uuid::nil().
    let mt = serde_json::json!({
        "name": "tmpl",
        "channel": "InApp",
        "body_template": "hello {{name}}"
    })
    .to_string();
    let (cmt, bmt) = api_post!("/api/messaging/templates", &mt, t).await;
    surface_route_hit(&mut n, "POST /api/messaging/templates", cmt, &bmt).await;
    let msg_tmpl_id = if cmt == 201 {
        common::assert_field("POST /api/messaging/templates", &bmt, "template.id");
        common::extract_uuid("POST /api/messaging/templates", &bmt, "template.id")
    } else if cmt == 409 {
        let (_, list_body) = common::get_json("/api/messaging/templates", t).await;
        common::find_in_list(&list_body, "name", "tmpl")
            .and_then(|item| {
                item.get("id")
                    .and_then(|id| id.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok())
            })
            .unwrap_or_else(|| Uuid::parse_str(DUMMY).unwrap())
    } else {
        Uuid::parse_str(DUMMY).unwrap()
    };

    let (c_mtl, b_mtl) = api_get!("/api/messaging/templates", t).await;
    surface_route_hit(&mut n, "GET /api/messaging/templates", c_mtl, &b_mtl).await;
    if c_mtl == 200 {
        common::assert_is_array("GET /api/messaging/templates", &b_mtl);
    }
    let mtid = msg_tmpl_id.to_string();
    let (cmg3, bmg3) = api_get!(&format!("/api/messaging/templates/{}", mtid), t).await;
    surface_route_hit(&mut n, "GET /api/messaging/templates/{id}", cmg3, &bmg3).await;
    if cmg3 == 200 {
        common::assert_field("GET /api/messaging/templates/{id}", &bmg3, "template.id");
    }
    let umt = serde_json::json!({}).to_string();
    let (cmu, bmu) = api_put!(&format!("/api/messaging/templates/{}", mtid), &umt, t).await;
    surface_route_hit(&mut n, "PUT /api/messaging/templates/{id}", cmu, &bmu).await;
    let (cmd, bmd) = api_delete!(&format!("/api/messaging/templates/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "DELETE /api/messaging/templates/{id}", cmd, &bmd).await;

    let tr = serde_json::json!({
        "name": "tr",
        "event": "Custom",
        "template_id": msg_tmpl_id
    })
    .to_string();
    let (ctr, btr) = api_post!("/api/messaging/triggers", &tr, t).await;
    surface_route_hit(&mut n, "POST /api/messaging/triggers", ctr, &btr).await;
    if ctr == 201 {
        common::assert_field("POST /api/messaging/triggers", &btr, "id");
    }
    let (c_trl, b_trl) = api_get!("/api/messaging/triggers", t).await;
    surface_route_hit(&mut n, "GET /api/messaging/triggers", c_trl, &b_trl).await;
    if c_trl == 200 {
        common::assert_is_array("GET /api/messaging/triggers", &b_trl);
    }
    let (ctd2, btd2) = api_delete!(&format!("/api/messaging/triggers/{}", DUMMY), t).await;
    surface_route_hit(&mut n, "DELETE /api/messaging/triggers/{id}", ctd2, &btd2).await;

    let fire = serde_json::json!({
        "event": "Custom",
        "payload": {}
    })
    .to_string();
    let (cfi, bfi) = api_post!("/api/messaging/fire", &fire, t).await;
    surface_route_hit(&mut n, "POST /api/messaging/fire", cfi, &bfi).await;

    let (c_nt, b_nt) = api_get!("/api/messaging/notifications", t).await;
    surface_route_hit(&mut n, "GET /api/messaging/notifications", c_nt, &b_nt).await;
    if c_nt == 200 {
        common::assert_field("GET /api/messaging/notifications", &b_nt, "notifications");
        common::assert_field("GET /api/messaging/notifications", &b_nt, "unread_count");
    }
    let (cnr, bnr) = api_post!(&format!("/api/messaging/notifications/{}/read", DUMMY), "{}", t).await;
    surface_route_hit(&mut n, "POST /api/messaging/notifications/{id}/read", cnr, &bnr).await;
    let (cnd, bnd) = api_post!(&format!("/api/messaging/notifications/{}/dismiss", DUMMY), "{}", t).await;
    surface_route_hit(&mut n, "POST /api/messaging/notifications/{id}/dismiss", cnd, &bnd).await;
    let (cnm, bnm) = api_post!("/api/messaging/notifications/read-all", "{}", t).await;
    surface_route_hit(&mut n, "POST /api/messaging/notifications/read-all", cnm, &bnm).await;

    let (c_pq, b_pq) = api_get!("/api/messaging/payloads", t).await;
    surface_route_hit(&mut n, "GET /api/messaging/payloads", c_pq, &b_pq).await;
    let (cpe, bpe) = api_post!("/api/messaging/payloads/export", &serde_json::json!({}).to_string(), t).await;
    surface_route_hit(&mut n, "POST /api/messaging/payloads/export", cpe, &bpe).await;
    let md = serde_json::json!({ "payload_ids": [] }).to_string();
    let (cmd3, bmd3) = api_post!("/api/messaging/payloads/mark-delivered", &md, t).await;
    surface_route_hit(&mut n, "POST /api/messaging/payloads/mark-delivered", cmd3, &bmd3).await;
    let mf = serde_json::json!({ "payload_id": DUMMY, "error": "x" }).to_string();
    let (cmf, bmf) = api_post!("/api/messaging/payloads/mark-failed", &mf, t).await;
    surface_route_hit(&mut n, "POST /api/messaging/payloads/mark-failed", cmf, &bmf).await;
    let (cdl, bdl) = api_get!(&format!("/api/messaging/payloads/{}/log", DUMMY), t).await;
    surface_route_hit(&mut n, "GET /api/messaging/payloads/{id}/log", cdl, &bdl).await;

    let (clo, blo) = api_post!("/api/auth/logout", "{}", t).await;
    surface_route_hit(&mut n, "POST /api/auth/logout", clo, &blo).await;

    assert_eq!(
        n as usize,
        route_catalog::API_INTEGRATION_HTTP_CALLS,
        "each Actix route needs one hit(); update tests/route_catalog.rs ROUTES and the 113 constant",
    );
}
