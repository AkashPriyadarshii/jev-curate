(function () {
  function fallbackCopy(text, done) {
    var ta = document.createElement('textarea');
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand('copy');
    document.body.removeChild(ta);
    done();
  }

  /* sonner: bottom-right copy feedback toast */
  var toastRegion = document.getElementById('toast-region');
  function showToast(msg) {
    if (!toastRegion) return;
    var el = document.createElement('div');
    el.className = 'toast';
    el.textContent = msg;
    toastRegion.appendChild(el);
    setTimeout(function () {
      if (el.parentNode) el.parentNode.removeChild(el);
    }, 2000);
  }

  /* copy button */
  var btn = document.querySelector('.copy-btn');
  var copyStatus = document.getElementById('copy-status');
  if (btn) {
    var origLabel = btn.textContent;
    var copyTimer = null;
    btn.addEventListener('click', function () {
      var text = btn.getAttribute('data-copy');
      var done = function () {
        clearTimeout(copyTimer);
        btn.textContent = 'copied';
        if (copyStatus) copyStatus.textContent = 'Copied to clipboard';
        showToast('Copied install command to clipboard');
        copyTimer = setTimeout(function () {
          btn.textContent = origLabel;
          if (copyStatus) copyStatus.textContent = '';
        }, 1600);
      };
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(text).then(done).catch(function () {
          fallbackCopy(text, done);
        });
      } else {
        fallbackCopy(text, done);
      }
    });
  }

  /* hamburger nav toggle */
  var toggle = document.querySelector('.nav-toggle');
  var mobileNav = document.querySelector('.nav-links');
  if (toggle && mobileNav) {
    function closeNav() {
      mobileNav.classList.remove('open');
      toggle.setAttribute('aria-expanded', 'false');
      toggle.setAttribute('aria-label', 'Open menu');
    }
    toggle.addEventListener('click', function () {
      var open = mobileNav.classList.toggle('open');
      toggle.setAttribute('aria-expanded', open ? 'true' : 'false');
      toggle.setAttribute('aria-label', open ? 'Close menu' : 'Open menu');
      if (open) {
        var firstLink = mobileNav.querySelector('a, button');
        if (firstLink) firstLink.focus();
      }
    });
    mobileNav.addEventListener('click', function (e) {
      if (e.target.closest('a')) closeNav();
    });
    /* close on outside click */
    document.addEventListener('click', function (e) {
      if (mobileNav.classList.contains('open') && !toggle.contains(e.target) && !mobileNav.contains(e.target)) {
        closeNav();
      }
    });
    /* close on Escape */
    document.addEventListener('keydown', function (e) {
      if (e.key === 'Escape' && mobileNav.classList.contains('open')) {
        closeNav();
        toggle.focus();
      }
    });
  }
  /* hero copy button toast */
  var heroCopyBtn = document.querySelector('.hero-copy-btn');
  if (heroCopyBtn) {
    heroCopyBtn.addEventListener('click', function () {
      showToast('Copied snippet to clipboard');
    });
  }

  /* radix-primitives: roving ArrowLeft/ArrowRight + Home/End on role="tablist" */
  document.querySelectorAll('[role="tablist"]').forEach(function (tablist) {
    tablist.addEventListener('keydown', function (e) {
      var tabs = Array.prototype.slice.call(tablist.querySelectorAll('[role="tab"]'));
      if (!tabs.length) return;
      var idx = tabs.indexOf(document.activeElement);
      if (idx === -1) return;
      var nextIdx = null;
      if (e.key === 'ArrowRight') nextIdx = (idx + 1) % tabs.length;
      else if (e.key === 'ArrowLeft') nextIdx = (idx - 1 + tabs.length) % tabs.length;
      else if (e.key === 'Home') nextIdx = 0;
      else if (e.key === 'End') nextIdx = tabs.length - 1;
      if (nextIdx !== null) {
        e.preventDefault();
        tabs[nextIdx].focus();
        tabs[nextIdx].click();
      }
    });
  });

  /* cmdk + ui: global '/' shortcut to focus #sandbox-input */
  document.addEventListener('keydown', function (e) {
    if (e.key !== '/' || e.metaKey || e.ctrlKey || e.altKey) return;
    var tag = e.target && e.target.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || (e.target && e.target.isContentEditable)) return;
    var sandboxInput = document.getElementById('sandbox-input');
    if (sandboxInput) {
      e.preventDefault();
      sandboxInput.focus();
    }
  });
})();