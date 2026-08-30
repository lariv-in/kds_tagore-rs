(function () {
  if (window.__kdsThemeBound) return;
  window.__kdsThemeBound = true;

  function initHeaderScroll() {
    var header = document.querySelector(".site-header, .gjs-navbar.site-header, #header");
    if (!header || header.dataset.kdsScrollBound) return;
    header.dataset.kdsScrollBound = "true";
    window.addEventListener("scroll", function () {
      header.classList.toggle("scrolled", window.scrollY > 40);
    });
  }

  function initMobileNav() {
    document.querySelectorAll('[data-gjs-type="p_website.navbar"], .gjs-navbar').forEach(function (nav) {
      var toggleBtn = nav.querySelector(".mobile-nav-toggle");
      var navMenu = nav.querySelector(".gjs-navbar-links");
      if (!toggleBtn || !navMenu || toggleBtn.dataset.bound) return;
      toggleBtn.dataset.bound = "true";
      toggleBtn.addEventListener("click", function (e) {
        e.stopPropagation();
        var open = !navMenu.classList.contains("open");
        navMenu.classList.toggle("open", open);
        toggleBtn.classList.toggle("open", open);
        toggleBtn.setAttribute("aria-expanded", String(open));
      });
      document.addEventListener("click", function (e) {
        if (!navMenu.contains(e.target) && !toggleBtn.contains(e.target)) {
          navMenu.classList.remove("open");
          toggleBtn.classList.remove("open");
          toggleBtn.setAttribute("aria-expanded", "false");
        }
      });
      navMenu.querySelectorAll("a").forEach(function (link) {
        link.addEventListener("click", function () {
          navMenu.classList.remove("open");
          toggleBtn.classList.remove("open");
          toggleBtn.setAttribute("aria-expanded", "false");
        });
      });
    });
  }

  function initExpandables() {
    var reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    var canHover = window.matchMedia("(hover: hover) and (pointer: fine)").matches;

    function setBodyOpen(body, open) {
      if (!body) return;
      body.style.height = open ? "auto" : "0px";
      body.style.opacity = open ? "1" : "0";
    }

    function syncToggle(section, open) {
      var toggle = section.querySelector(".expand-toggle");
      if (!toggle) return;
      toggle.setAttribute("aria-expanded", open ? "true" : "false");
    }

    function animateOpen(section) {
      if (section._expandOpen) return;
      section._expandOpen = true;
      var body = section.querySelector(".expand-body");
      section.classList.add("is-open");
      syncToggle(section, true);
      if (!body) return;
      if (reduceMotion) {
        setBodyOpen(body, true);
        return;
      }
      var start = body.getBoundingClientRect().height;
      body.style.height = start + "px";
      body.style.opacity = start > 1 ? getComputedStyle(body).opacity : "0";
      body.offsetHeight;
      body.style.height = body.scrollHeight + "px";
      body.style.opacity = "1";
      body.addEventListener("transitionend", function onEnd(e) {
        if (e.propertyName !== "height") return;
        if (section._expandOpen) body.style.height = "auto";
        body.removeEventListener("transitionend", onEnd);
      });
    }

    function animateClose(section) {
      if (!section._expandOpen) return;
      section._expandOpen = false;
      var body = section.querySelector(".expand-body");
      section.classList.remove("is-open");
      syncToggle(section, false);
      if (!body) return;
      if (reduceMotion) {
        setBodyOpen(body, false);
        return;
      }
      var start = body.getBoundingClientRect().height || body.scrollHeight;
      body.style.height = start + "px";
      body.offsetHeight;
      body.style.height = "0px";
      body.style.opacity = "0";
    }

    document.querySelectorAll(".expand-section, .machine-card").forEach(function (section) {
      if (section.dataset.kdsExpandBound) return;
      var hdr = section.querySelector(".expand-header");
      var body = section.querySelector(".expand-body");
      if (!hdr || !body) return;
      section.dataset.kdsExpandBound = "true";
      section._expandOpen = section.classList.contains("is-open");
      if (!section._expandOpen) setBodyOpen(body, false);
      syncToggle(section, section._expandOpen);
      hdr.addEventListener("click", function () {
        if (canHover) return;
        if (section._expandOpen) animateClose(section);
        else animateOpen(section);
      });
      if (canHover) {
        section.addEventListener("mouseenter", function () {
          animateOpen(section);
        });
        section.addEventListener("mouseleave", function () {
          animateClose(section);
        });
      }
    });

    function openExpandFromHash(animate) {
      var id = location.hash.slice(1);
      if (!id) return;
      var section = document.getElementById(id);
      if (!section || !section.classList.contains("expand-section")) return;
      if (section._expandOpen) return;
      if (animate) animateOpen(section);
      else {
        section._expandOpen = true;
        section.classList.add("is-open");
        setBodyOpen(section.querySelector(".expand-body"), true);
        syncToggle(section, true);
      }
    }

    openExpandFromHash(false);
    if (!window.__kdsHashExpandBound) {
      window.__kdsHashExpandBound = true;
      window.addEventListener("hashchange", function () {
        openExpandFromHash(true);
      });
    }
  }

  function boot() {
    initHeaderScroll();
    initMobileNav();
    initExpandables();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
