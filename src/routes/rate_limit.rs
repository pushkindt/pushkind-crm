//! IP-based rate limiting helpers for storefront OTP endpoints.

use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use actix_web::HttpRequest;

pub const MAX_REQUESTS: u32 = 10;
pub const WINDOW_SECONDS: u64 = 60;
pub const TRUST_FORWARDED_HEADERS: bool = false;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitExceeded {
    pub ip: IpAddr,
    pub retry_after: Duration,
}

#[derive(Debug)]
pub struct StoreOtpIpRateLimiter {
    state: Mutex<RateLimitState>,
}

#[derive(Debug, Default)]
struct RateLimitState {
    buckets: HashMap<IpAddr, VecDeque<Instant>>,
    last_global_cleanup: Option<Instant>,
}

impl StoreOtpIpRateLimiter {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(RateLimitState::default()),
        }
    }

    pub fn check(&self, req: &HttpRequest) -> Result<(), RateLimitExceeded> {
        let Some(ip) = extract_client_ip(req) else {
            return Ok(());
        };

        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let now = Instant::now();

        Self::check_ip_with_state(ip, now, &mut guard)
    }

    fn check_ip_with_state(
        ip: IpAddr,
        now: Instant,
        state: &mut RateLimitState,
    ) -> Result<(), RateLimitExceeded> {
        let window = Duration::from_secs(WINDOW_SECONDS);
        let max_requests = MAX_REQUESTS as usize;

        let should_cleanup = state
            .last_global_cleanup
            .is_none_or(|last| now.saturating_duration_since(last) >= window);

        if should_cleanup {
            cleanup_stale_buckets(state, now, window);
            state.last_global_cleanup = Some(now);
        }

        let entries = state.buckets.entry(ip).or_default();
        prune_bucket(entries, now, window);

        if entries.len() >= max_requests {
            let retry_after = entries
                .front()
                .map(|&oldest| window.saturating_sub(now.saturating_duration_since(oldest)))
                .unwrap_or(window);

            return Err(RateLimitExceeded { ip, retry_after });
        }

        entries.push_back(now);

        Ok(())
    }
}

fn extract_client_ip(req: &HttpRequest) -> Option<IpAddr> {
    if TRUST_FORWARDED_HEADERS
        && let Some(real_ip) = req.connection_info().realip_remote_addr()
        && let Some(ip) = parse_forwarded_ip(real_ip)
    {
        return Some(ip);
    }

    req.peer_addr().map(|addr| addr.ip())
}

impl Default for StoreOtpIpRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

fn cleanup_stale_buckets(state: &mut RateLimitState, now: Instant, window: Duration) {
    for entries in state.buckets.values_mut() {
        prune_bucket(entries, now, window);
    }
    state.buckets.retain(|_, entries| !entries.is_empty());
}

fn prune_bucket(entries: &mut VecDeque<Instant>, now: Instant, window: Duration) {
    while let Some(&front) = entries.front() {
        if now.saturating_duration_since(front) >= window {
            entries.pop_front();
        } else {
            break;
        }
    }
}

fn parse_forwarded_ip(raw_value: &str) -> Option<IpAddr> {
    let first_value = raw_value.split(',').next()?.trim();
    let unprefixed = first_value
        .strip_prefix("for=")
        .unwrap_or(first_value)
        .trim_matches('"');

    if let Ok(addr) = unprefixed.parse::<IpAddr>() {
        return Some(addr);
    }
    if let Ok(addr) = unprefixed.parse::<SocketAddr>() {
        return Some(addr.ip());
    }

    None
}
