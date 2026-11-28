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

  /* sonner: bottom-right copy feedback toast (@starting-style + .is-leaving) */
  var toastRegion = document.getElementById('toast-region');
  function showToast(msg) {
    if (!toastRegion) return;
    var el = document.createElement('div');
    el.className = 'toast';
    el.textContent = msg;
    toastRegion.appendChild(el);
    setTimeout(function () {
      el.classList.add('is-leaving');
      setTimeout(function () {
        if (el.parentNode) el.parentNode.removeChild(el);
      }, 220);
    }, 1800);
  }

  function copyWithFeedback(text, onDone) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(onDone).catch(function () {
        fallbackCopy(text, onDone);
      });
    } else {
      fallbackCopy(text, onDone);
    }
  }

  /* hero 1-click install bar (pip / cargo toggle) */
  var installCmdEl = document.getElementById('hero-install-cmd');
  var installCopyBtn = document.getElementById('hero-install-copy');
  var installPkgBtns = document.querySelectorAll('.install-pkg[data-cmd]');
  var copyStatus = document.getElementById('copy-status');

  if (installCmdEl && installPkgBtns.length) {
    installPkgBtns.forEach(function (pkgBtn) {
      pkgBtn.addEventListener('click', function () {
        installPkgBtns.forEach(function (b) {
          var active = b === pkgBtn;
          b.classList.toggle('is-active', active);
          b.setAttribute('aria-pressed', active ? 'true' : 'false');
        });
        installCmdEl.textContent = pkgBtn.getAttribute('data-cmd');
      });
    });
  }

  if (installCopyBtn && installCmdEl) {
    var installLabel = installCopyBtn.querySelector('.copy-label') || installCopyBtn;
    var installOrig = installLabel.textContent;
    var installTimer = null;
    installCopyBtn.addEventListener('click', function () {
      var text = installCmdEl.textContent.trim();
      copyWithFeedback(text, function () {
        clearTimeout(installTimer);
        installCopyBtn.classList.remove('is-copied');
        void installCopyBtn.offsetWidth;
        installCopyBtn.classList.add('is-copied');
        installLabel.textContent = 'copied';
        if (copyStatus) copyStatus.textContent = 'Copied to clipboard';
        showToast('Copied install command to clipboard');
        installTimer = setTimeout(function () {
          installCopyBtn.classList.remove('is-copied');
          installLabel.textContent = installOrig;
          if (copyStatus) copyStatus.textContent = '';
        }, 1600);
      });
    });
  }

  /* copy button */
  var btn = document.querySelector('.copy-btn');
  if (btn) {
    var origLabel = btn.textContent;
    var copyTimer = null;
    btn.addEventListener('click', function () {
      var text = btn.getAttribute('data-copy');
      copyWithFeedback(text, function () {
        clearTimeout(copyTimer);
        btn.classList.remove('is-copied');
        void btn.offsetWidth;
        btn.classList.add('is-copied');
        btn.textContent = 'copied';
        if (copyStatus) copyStatus.textContent = 'Copied to clipboard';
        showToast('Copied install command to clipboard');
        copyTimer = setTimeout(function () {
          btn.classList.remove('is-copied');
          btn.textContent = origLabel;
          if (copyStatus) copyStatus.textContent = '';
        }, 1600);
      });
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