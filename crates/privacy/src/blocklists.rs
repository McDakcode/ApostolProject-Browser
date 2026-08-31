// Made by MrDuck && Ox-Alpha
//! Built-in blocklists shipped with the binary (§10A.11): network-layer
//! domain rules plus generic cosmetic-filtering CSS. Protection works
//! offline on first launch; bigger/custom lists come via «Мои списки».
//!
//! Curation notes: every entry is a dedicated ad/tracker/fingerprint/
//! malware infrastructure domain — never a whole publisher/mail/social
//! domain (blocking `mail.ru` or `yandex.ru` outright would break the
//! web). Subdomains match automatically (`TrackerBlocker::classify`).

use crate::TrackerCategory;

/// Network-layer domain rules (domain → category).
pub fn builtin_rules() -> Vec<(&'static str, TrackerCategory)> {
    use TrackerCategory::*;
    vec![
        // ---------------- Analytics ----------------
        ("google-analytics.com", Analytics),
        ("googletagmanager.com", Analytics),
        ("analytics.google.com", Analytics),
        ("segment.io", Analytics),
        ("segment.com", Analytics),
        ("mixpanel.com", Analytics),
        ("amplitude.com", Analytics),
        ("heap.io", Analytics),
        ("hotjar.com", Analytics),
        ("mouseflow.com", Analytics),
        ("fullstory.com", Analytics),
        ("matomo.cloud", Analytics),
        ("statcounter.com", Analytics),
        ("quantserve.com", Analytics),
        ("scorecardresearch.com", Analytics),
        ("chartbeat.com", Analytics),
        ("clicky.com", Analytics),
        ("mc.yandex.ru", Analytics),
        ("clarity.ms", Analytics),
        ("bat.bing.com", Analytics),
        ("liveinternet.ru", Analytics),
        ("yadro.ru", Analytics),
        ("hotlog.ru", Analytics),
        ("onthe.io", Analytics),
        ("histats.com", Analytics),
        // ---------------- Advertising ----------------
        ("doubleclick.net", Advertising),
        ("googlesyndication.com", Advertising),
        ("googleadservices.com", Advertising),
        ("googletagservices.com", Advertising),
        ("adservice.google.com", Advertising),
        ("adtrafficquality.google", Advertising),
        ("adnxs.com", Advertising),
        ("appnexus.com", Advertising),
        ("adsystem.com", Advertising),
        ("amazon-adsystem.com", Advertising),
        ("criteo.com", Advertising),
        ("criteo.net", Advertising),
        ("taboola.com", Advertising),
        ("taboolasyndication.com", Advertising),
        ("outbrain.com", Advertising),
        ("rubiconproject.com", Advertising),
        ("pubmatic.com", Advertising),
        ("openx.net", Advertising),
        ("casalemedia.com", Advertising),
        ("smartadserver.com", Advertising),
        ("adform.net", Advertising),
        ("yieldmo.com", Advertising),
        ("sharethrough.com", Advertising),
        ("33across.com", Advertising),
        ("bidswitch.net", Advertising),
        ("teads.tv", Advertising),
        ("media.net", Advertising),
        ("revcontent.com", Advertising),
        ("mgid.com", Advertising),
        ("adskeeper.com", Advertising),
        ("propellerads.com", Advertising),
        ("propellerclick.com", Advertising),
        ("popads.net", Advertising),
        ("popcash.net", Advertising),
        ("clickadu.com", Advertising),
        ("hilltopads.net", Advertising),
        ("admaven.com", Advertising),
        ("exoclick.com", Advertising),
        ("juicyads.com", Advertising),
        ("trafficstars.com", Advertising),
        ("trafficjunky.net", Advertising),
        ("adroll.com", Advertising),
        ("indexww.com", Advertising),
        ("yieldlab.net", Advertising),
        ("improvedigital.com", Advertising),
        ("gumgum.com", Advertising),
        ("sonobi.com", Advertising),
        ("zedo.com", Advertising),
        ("bidr.io", Advertising),
        ("liadm.com", Advertising),
        ("id5-sync.com", Advertising),
        ("adsco.re", Advertising),
        ("rtbhouse.com", Advertising),
        ("emxdgt.com", Advertising),
        ("adsafeprotected.com", Advertising),
        ("doubleverify.com", Advertising),
        ("spotxchange.com", Advertising),
        ("spotx.tv", Advertising),
        ("smartclip.net", Advertising),
        ("springserve.com", Advertising),
        ("freewheel.tv", Advertising),
        ("fwmrm.net", Advertising),
        ("innovid.com", Advertising),
        ("adition.com", Advertising),
        ("zemanta.com", Advertising),
        ("addthis.com", Advertising),
        ("sharethis.com", Advertising),
        // --- RU / CIS adtech (видны в живых логах APB) ---
        ("adfox.ru", Advertising),
        ("adriver.ru", Advertising),
        ("buzzoola.com", Advertising),
        ("sape.ru", Advertising),
        ("acint.net", Advertising),
        ("adhigh.net", Advertising),
        ("betweendigital.com", Advertising),
        ("hybrid.ai", Advertising),
        ("bumlam.com", Advertising),
        ("adfinity.pro", Advertising),
        ("adtec.ru", Advertising),
        ("otm-r.com", Advertising),
        ("videonow.ru", Advertising),
        ("smi2.net", Advertising),
        ("kadam.net", Advertising),
        ("traforet.com", Advertising),
        ("mradx.net", Advertising),
        ("an.yandex.ru", Advertising),
        ("ymatuhin.ru", Advertising),
        ("awaps.yandex.net", Advertising),
        ("yandexadexchange.net", Advertising),
        // ---------------- Social trackers ----------------
        ("facebook.net", Social),
        ("connect.facebook.net", Social),
        ("platform.twitter.com", Social),
        ("syndication.twitter.com", Social),
        ("cdn.syndication.twimg.com", Social),
        ("platform.linkedin.com", Social),
        ("snap.licdn.com", Social),
        ("assets.pinterest.com", Social),
        ("events.redditmedia.com", Social),
        ("static.ads-twitter.com", Social),
        ("top-fwz1.mail.ru", Social),
        // ---------------- Fingerprinting scripts ----------------
        ("fingerprintjs.com", Fingerprinting),
        ("fingerprint.com", Fingerprinting),
        ("fpjs.io", Fingerprinting),
        ("fptls.com", Fingerprinting),
        ("iovation.com", Fingerprinting),
        ("threatmetrix.com", Fingerprinting),
        ("perimeterx.net", Fingerprinting),
        ("px-cdn.net", Fingerprinting),
        ("distiltag.com", Fingerprinting),
        ("bluekai.com", Fingerprinting),
        ("krxd.net", Fingerprinting),
        ("demdex.net", Fingerprinting),
        ("omtrdc.net", Fingerprinting),
        ("everesttech.net", Fingerprinting),
        // ---------------- Malicious / deceptive ----------------
        ("coinhive.com", Malicious),
        ("authedmine.com", Malicious),
        ("cryptoloot.pro", Malicious),
        ("jsecoin.com", Malicious),
        ("mineralt.io", Malicious),
        ("coinhave.com", Malicious),
        ("coinerra.com", Malicious),
        ("webmine.cz", Malicious),
        ("coinimp.com", Malicious),
    ]
}

/// Generic cosmetic-filter CSS (§10A.11): hides leftover ad containers of
/// the networks we block at the network layer, so pages don't show empty
/// boxes. Deliberately CONSERVATIVE — selectors are tied to well-known
/// ad-network DOM hooks (AdSense/GPT/Yandex RTB/RU networks) or explicit
/// ad-convention class/id shapes, not arbitrary substrings like "ad".
pub const COSMETIC_CSS: &str = r#"
ins.adsbygoogle{display:none!important}
ins[data-ad-client]{display:none!important}
iframe[src*="doubleclick.net"],iframe[src*="googlesyndication"],
iframe[src*="googletagservices"],iframe[src*="adtrafficquality.google"],
iframe[src*="adriver.ru"],iframe[src*="adfox.ru"],iframe[src*="buzzoola"],
iframe[src*="videonow.ru"],iframe[src*="mgid.com"],iframe[src*="taboola.com"],
iframe[src*="outbrain.com"],iframe[src*="hilltopads"],iframe[src*="propellerads"],
iframe[src*="exoclick"],iframe[src*="clickadu"],iframe[src*="smi2.net"]{display:none!important}
[id^="yandex_rtb"],[id^="adfox_"],div[id^="adfox"],div[id*="ScriptRoot"],
[data-ad-slot],[data-ad-unit],[data-adv-id]{display:none!important}
[id^="ad-"],[class^="ad-"],[class^="ads-"],[class*=" ads-"],[class*=" ad-"],
[id$="-ads"],[id$="_ads"],div[class$="-ads"],div[class$="_ads"],
[aria-label="Advertisement"],[aria-label="Реклама"],[data-testid^="ad-"]{display:none!important}
"#;

/// Aggressive cosmetic CSS: layered on top of `COSMETIC_CSS` in the single
/// always-on ad mode. Far broader heuristics — ad-convention
/// substrings in classes/ids, banner/sponsor/promo wrappers, media elements
/// hosted in ad containers, and inline scripts that lace ads. Deliberately
/// may break an innocent page; that is the accepted trade-off of "maximum".
pub const AGGRESSIVE_CSS: &str = r#"
[id*="-ad-"],[id*="_ad_"],[class*="-ad-"],[class*="_ad_"],
[id*="advert"],[class*="advert"],[id*="adslot"],[class*="adslot"],
[id*="adunit"],[class*="adunit"],[id*="adsense"],[class*="adsense"],
[id*="sponsor"],[class*="sponsor"],[id*="promo"],[class*="promo"],
[id*="partner-post"],[class*="partner-post"],[class*="ad-container"],
[id*="ad-banner"],[class*="ad-banner"],[class*="banner-ad"],
[id*="top-banner"],[class*="top-banner"],[class*="sticky-ad"],
[id*="sidebar-ad"],[class*="sidebar-ad"],[class*="advertorial"],
[id*="ad-overlay"],[class*="ad-overlay"],[class*="pop-under"],
[id*="popup-ad"],[class*="popup-ad"],[class*="interstitial"],
[id*="ad-frame"],[class*="ad-frame"],[data-ad-client],[data-ad-slot],
[data-ad],[data-ads],[data-advert],[data-nosnippet]{display:none!important}
/* Banner/sponsor/promo textbook wrappers */
div.banner,section.banner,aside.banner,div.sponsor,section.sponsor,
div.promo,section.promo,div.partner,section.partner,
[class*="sponsored"],[id*="sponsored"],[class*="advertisement"],
[id*="advertisement"],.adsbygoogle{display:none!important}
/* Media that ships inside ad-shaped wrappers: gif/flash/iframe banners */
img[src*=".gif"][src*="/ad"],img[src*=".swf"],object[type="application/x-shockwave-flash"],
embed[type="application/x-shockwave-flash"],object[classid*="clsid:D27"][classid*=":flash"],
object[data*=".swf"],embed[src*=".swf"],iframe[src*="/ad/"],iframe[src*="adsense"],
iframe[src*="doubleclick"],iframe[src*="googlesyndication"],iframe[src*="adfox"],
iframe[src*="mgid"],iframe[src*="taboola"],iframe[src*="outbrain"]{display:none!important}
/* Banner creative paths & media — AdBlock-grade, catches same-origin
   /banners/ gif/png/flash test/real placements the host filter misses. */
img[src*="/banners/"],img[src*="/banner/"],img[src*="/ads/"],img[src*="/advert"],
img[src*="/ad-banner"],img[src*="/adbanner"],img[src*="ad_banner"],
object[data*="/banners/"],object[data*="/banner/"],object[data*=".swf"],
embed[src*="/banners/"],embed[src*=".swf"],iframe[src*="/banners/"]{display:none!important}
"#;

/// Cosmetic stylesheet-planter (internal): injects `<style id=apb-cosmetic>`
/// at document-start, re-planted if detached. Pure CSS matching means
/// dynamically inserted ad nodes are hidden automatically.
fn cosmetic_style_script(css: &str) -> String {
    format!(
        r#"(() => {{
  try {{
    const s = document.createElement("style");
    s.id = "apb-cosmetic";
    s.textContent = {css};
    const mount = () => {{
      const root = document.head || document.documentElement;
      if (!root || document.getElementById("apb-cosmetic")) return;
      root.appendChild(s);
    }};
    mount();
    if (!s.isConnected) {{
      new MutationObserver((_, obs) => {{
        if (s.isConnected) {{ obs.disconnect(); return; }}
        mount();
        if (s.isConnected) obs.disconnect();
      }}).observe(document, {{ childList: true, subtree: true }});
    }}
  }} catch (e) {{}}
}})();"#,
        css = serde_json::to_string(css).unwrap_or_else(|_| "\"\"".into())
    )
}

/// Full initialization script planting the cosmetic stylesheet at
/// document-start for the Balanced (compatibility-first) ad level.
pub fn cosmetic_filter_script() -> String {
    cosmetic_style_script(COSMETIC_CSS)
}

/// Aggressive DOM-sweeper (document-start, one MutationObserver). Goes a
/// step further than pure CSS: *removes* nodes whose class/id match ad
/// conventions and aborts the load of media-shaniped ad banners (gif/swf /
/// flash / ad iframes) by blanking their URL. Removing (not just hiding)
/// satisfies testers that probe for the element in `document`. Unknown
/// whitespace/innocent-looking classes are left alone.
fn aggressive_dom_script() -> String {
    r#"(() => {
  try {
    if (window.__apbAggAd) return;
    Object.defineProperty(window, "__apbAggAd", { value: true });
    const AD_CLASS_RE = /(^|[-_\s])(ad|ads|advert|adslot|adunit|adsense|banner|sponsor|promo|partner)(\d*)(\b|[-_\s]|$)/i;
    const AD_ID_RE = /(^|[-_])(ad|ads|advert|adslot|adunit|adsense|banner|sponsor|promo)(\d*)(\b|[-_])/i;
    const AD_HOST_RE = /\b(ad|ads|banner|sponsor|promo|advert|adsense|doubleclick|googlesyndication|adfox|mgid|taboola|outbrain|propeller|popads|criteo|taboola)([._-].*)?\./i;
    const adish = (n) => {
      if (!n || n.nodeType !== 1) return false;
      const id = n.id || "";
      if (id && AD_ID_RE.test(id)) return true;
      const cls = (n.className && typeof n.className === "string") ? n.className : "";
      if (cls && AD_CLASS_RE.test(cls)) {
        // Skip innocent "admin", "addition" style substrings via word filters.
        if (/\bad(admin|dress|dition|venture|apt|obe|ult|verb|dorable|visor)|\badmin\b/i.test(cls)) return false;
        return true;
      }
      return false;
    };
    const killEl = (n) => { try { n.remove ? n.remove() : n.parentNode && n.parentNode.removeChild(n); } catch (e) {} };
    const killMedia = (n) => {
      try {
        if (!n || n.nodeType !== 1) return;
        const raw = n.src || n.data || "";
        const src = (raw || "").toLowerCase();
        const hostish = AD_HOST_RE.test(src);
        const pathish = /(\/ad\/|\/ads\/|\/banner\/?|\/adsense|\/pagead\/|\/adframe|\/advert)/.test(src);
        const adfile = /\.swf$/.test(src) || /\/(a[d]|banner)[^\/]*\.(gif|png|jpg|jpeg|webp)$/.test(src);
        if (hostish || pathish || adfile) { killEl(n); return; }
        // Empty-source media nodes never carry real content (spacer gifs /
        // already-blanked placeholders) — dropping them removes the box.
        if (!raw && /^(IMG|OBJECT|EMBED|IFRAME)$/.test(n.tagName)) killEl(n);
      } catch (e) {}
    };
    const sweep = () => {
      if (document.body) {
        document.querySelectorAll("img,iframe,object,embed").forEach(killMedia);
      }
      if (document.body) {
        document.querySelectorAll('div,section,aside,ins,iframe,object,embed').forEach((n) => { if (adish(n)) killEl(n); });
      }
    };
    sweep();
    const obs = new MutationObserver(() => {
      try { const m = (mutation) => { for (const nn of mutation.addedNodes || []) { if (nn.nodeType === 1) { if (adish(nn)) killEl(nn); else nn.querySelectorAll && nn.querySelectorAll('img,iframe,object,embed,div,section,aside,ins').forEach((c) => { if (adish(c)) killEl(c); else killMedia(c); }); } } }; obs.takeRecords().forEach(m); } catch (e) {}
    });
    obs.observe(document.documentElement || document, { childList: true, subtree: true });
    setInterval(sweep, 1500);
  } catch (e) {}
})();"#
        .to_string()
}

/// Full AdBlock-style init script (single always-on mode): the wide
/// stylesheet + the DOM-sweeper above, shipped together as one injection.
pub fn aggressive_filter_script() -> String {
    format!(
        "{}{}",
        cosmetic_style_script(AGGRESSIVE_CSS),
        aggressive_dom_script()
    )
}

// ---------------------------------------------------------------------------
// Filter-list ingestion (hosts / AdGuard-DNS / ABP domain subset)
// ---------------------------------------------------------------------------

/// Extract plain domains from the formats real-world filter lists use:
/// hosts-file lines (`0.0.0.0 ads.example`), bare domains,
/// `||domain^` ABP/AdGuard network rules (options after `$` are dropped —
/// we approximate them as domain blocks; honest boundary), `domain^`.
/// Cosmetic rules (`##…`, `#@#…`), allow-lists (`@@…`) and comments are
/// skipped. Output is lowercase, deduplicated, order-preserving.
pub fn extract_domains(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('!') || l.starts_with('#') {
            continue;
        }
        if l.contains("##") || l.contains("#@#") || l.starts_with("@@") {
            continue;
        }
        let mut tok = l.split_whitespace();
        let first = match tok.next() {
            Some(f) => f,
            None => continue,
        };
        // hosts-file entry: IP + one or more hostnames (take the first).
        if first.parse::<std::net::IpAddr>().is_ok() {
            if let Some(d) = tok.next() {
                push_domain(&mut out, &mut seen, d);
            }
            continue;
        }
        // ABP/AdGuard: ||domain^$options — take up to the first anchor.
        if let Some(rest) = l.strip_prefix("||") {
            let end = rest.find(['^', '$', '/', ':']).unwrap_or(rest.len());
            push_domain(&mut out, &mut seen, &rest[..end]);
            continue;
        }
        // Bare domain, possibly with ^ anchor or $options glued on.
        let mut d = first;
        for cut in ['^', '$', '/'] {
            if let Some(i) = d.find(cut) {
                d = &d[..i];
            }
        }
        push_domain(&mut out, &mut seen, d);
    }
    out
}

fn push_domain(out: &mut Vec<String>, seen: &mut std::collections::HashSet<String>, raw: &str) {
    let d = raw.trim().trim_start_matches('.').trim_end_matches('.').to_lowercase();
    if crate::is_plausible_domain(&d) && seen.insert(d.clone()) {
        out.push(d);
    }
}

// ---------------------------------------------------------------------------
// In-page request blocker (the extension-grade layer)
// ---------------------------------------------------------------------------
//
// The proxy sees only CONNECT authorities for HTTPS traffic, so dynamic
// beacons sent by page JS to first-party paths (`/collect`, `/telemetry`)
// are invisible to it. This shim — injected into every frame like the
// fingerprint script — patches sendBeacon/fetch/XHR and aborts requests
// whose URL matches a curated pattern list. Patterns stay small (~200):
// the heavy lifting for third-party domains remains at the proxy.

const REQUEST_TOKENS: &[&str] = &[
    "/ads/", "/adframe", "/advert", "/banner/", "/pagead/", "/adsystem",
    "/analytics.js", "/gtag/js", "metrika/tag.js", "/telemetry", "/collect?v=",
    "/beacon.gif", "__utm", "/pixel?", "/track?", "/event?",
];

/// URL substrings the in-page shim should treat as tracking/ad requests.
pub fn builtin_request_patterns() -> Vec<String> {
    let mut v: Vec<String> = builtin_rules()
        .into_iter()
        .filter(|(_, c)| {
            matches!(
                c,
                TrackerCategory::Analytics
                    | TrackerCategory::Advertising
                    | TrackerCategory::Fingerprinting
            )
        })
        .map(|(d, _)| d.to_string())
        .collect();
    v.extend(REQUEST_TOKENS.iter().map(|s| s.to_string()));
    v.sort();
    v.dedup();
    v
}

/// Extra URL substrings the in-page shim always aborts — banner creative
/// servers, flash/media placements and generic ad-served paths beyond the
/// curated core set.
const AGGRESSIVE_REQUEST_TOKENS: &[&str] = &[
    "/ads/", "/ad/", "/adserver", "/advert", "/adbanner", "/adframe", "/adunit",
    "/adsense", "/pagead/", "/dfp/", "/gpt/ad", "/yandex_rtb", "/an.yandex",
    "/creativ", "/creative/", "/banner/", "/banner?", "/banner.", ".swf",
    "/sponsor", "/promo/ad", "/popunder", "/popunders", "/pop_exit", "/exitad",
];

/// URL substrings for the in-page shim in Aggressive mode: the balanced set
/// plus aggressive tokens. May over-match (that's the point of the level).
pub fn builtin_request_patterns_aggressive() -> Vec<String> {
    let mut v = builtin_request_patterns();
    v.extend(AGGRESSIVE_REQUEST_TOKENS.iter().map(|s| s.to_string()));
    v.sort();
    v.dedup();
    v
}

/// Initialization script patching beacon/fetch/XHR with pattern matching.
pub fn request_blocker_script(patterns: &[String]) -> String {
    let json = serde_json::to_string(patterns).unwrap_or_else(|_| "[]".into());
    format!(
        r#"(() => {{
  try {{
    if (window.__apbReqBlock) return;
    Object.defineProperty(window, "__apbReqBlock", {{ value: true }});
    const PAT = {json};
    const hit = (u) => {{
      if (!u) return false;
      const url = String(u).toLowerCase();
      for (let i = 0; i < PAT.length; i++) if (url.includes(PAT[i])) return true;
      return false;
    }};
    const abort = () => new DOMException("APB request blocked", "AbortError");
    if (navigator.sendBeacon) {{
      const nb = navigator.sendBeacon.bind(navigator);
      navigator.sendBeacon = (u, d) => hit(u) ? false : nb(u, d);
    }}
    const of = window.fetch;
    if (of) window.fetch = (input, init) => {{
      const u = typeof input === "string" ? input : (input && input.url) || "";
      return hit(u) ? Promise.reject(abort()) : of.apply(window, [input, init]);
    }};
    const xo = XMLHttpRequest.prototype.open;
    XMLHttpRequest.prototype.open = function (m, u, ...rest) {{
      this.__apbBlocked = hit(u);
      return xo.apply(this, [m, u, ...rest]);
    }};
    const xs = XMLHttpRequest.prototype.send;
    XMLHttpRequest.prototype.send = function (...a) {{
      if (this.__apbBlocked) throw abort();
      return xs.apply(this, a);
    }};
  }} catch (e) {{}}
}})();"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_cover_major_networks_including_ru() {
        let rules = builtin_rules();
        assert!(rules.len() > 140, "blocklist should stay substantial");
        let get = |d: &str| {
            rules
                .iter()
                .find(|(dom, _)| *dom == d)
                .map(|(_, c)| *c)
                .expect(d)
        };
        assert_eq!(get("adfox.ru"), TrackerCategory::Advertising);
        assert_eq!(get("adriver.ru"), TrackerCategory::Advertising);
        assert_eq!(get("mc.yandex.ru"), TrackerCategory::Analytics);
        assert_eq!(get("coinimp.com"), TrackerCategory::Malicious);
        // Whole-publisher domains must never appear (would break the web).
        for banned in ["yandex.ru", "mail.ru", "vk.com", "google.com", "youtube.com"] {
            assert!(rules.iter().all(|(dom, _)| *dom != banned));
        }
    }

    #[test]
    fn cosmetic_script_embeds_css_and_guards_double_mount() {
        let js = cosmetic_filter_script();
        assert!(js.contains("adsbygoogle"));
        assert!(js.contains("apb-cosmetic"));
        assert!(js.contains("MutationObserver"));
    }

    #[test]
    fn extract_domains_parses_hosts_and_abp_syntax() {
        let text = "! Title\n0.0.0.0 ads.one.example\n127.0.0.1 tracker.two.example\n\
                    ||ads.three.example^\n||four.example$script,image\nplain.five.example^/\n\
                    @@||allow.example^\nbanner##.ad-class\n# comment\n\nnot_a_domain\n";
        let d = extract_domains(text);
        assert_eq!(
            d,
            vec![
                "ads.one.example",
                "tracker.two.example",
                "ads.three.example",
                "four.example",
                "plain.five.example"
            ]
        );
    }

    #[test]
    fn request_patterns_cover_builtin_networks_and_tokens() {
        let p = builtin_request_patterns();
        assert!(p.iter().any(|x| x == "doubleclick.net"));
        assert!(p.iter().any(|x| x == "mc.yandex.ru"));
        assert!(p.iter().any(|x| x == "/pagead/"));
        // No malicious/social domains needed at the JS layer.
        assert!(!p.iter().any(|x| x == "coinhive.com"));
        let js = request_blocker_script(&p);
        assert!(js.contains("__apbReqBlock"));
        assert!(js.contains("sendBeacon"));
    }

    #[test]
    fn aggressive_scripts_are_substantial() {
        let js = aggressive_filter_script();
        assert!(js.contains("apb-cosmetic"));
        assert!(js.contains("__apbAggAd"));
        assert!(js.contains("MutationObserver"));
        assert!(js.contains("application/x-shockwave-flash"));
        let base = builtin_request_patterns();
        let agg = builtin_request_patterns_aggressive();
        assert!(agg.len() > base.len());
        assert!(agg.iter().any(|x| x == "doubleclick.net"));
        assert!(agg.iter().any(|x| x.contains("/creative")));
        assert!(agg.iter().any(|x| x == ".swf"));
    }
}
