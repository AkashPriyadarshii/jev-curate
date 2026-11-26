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

  /* copy button */
  var btn = document.querySelector('.copy-btn');
  var copyStatus = document.getElementById('copy-status');
  if (btn) {
    var origLabel = btn.textContent;
    btn.addEventListener('click', function () {
      var text = btn.getAttribute('data-copy');
      var done = function () {
        btn.textContent = 'copied';
        if (copyStatus) copyStatus.textContent = 'Copied to clipboard';
        setTimeout(function () {
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
      if (!toggle.contains(e.target) && !mobileNav.contains(e.target)) {
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
})();