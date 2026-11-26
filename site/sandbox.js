(function () {
  function wireTabs(tabs, onSelect) {
    if (!tabs.length) return;
    function activate(tab) {
      tabs.forEach(function (t) {
        var active = t === tab;
        t.classList.toggle('is-active', active);
        t.setAttribute('aria-selected', active ? 'true' : 'false');
        t.setAttribute('tabindex', active ? '0' : '-1');
      });
      onSelect(tab);
    }
    tabs.forEach(function (tab, idx) {
      var isActive = tab.classList.contains('is-active');
      tab.setAttribute('aria-selected', isActive ? 'true' : 'false');
      tab.setAttribute('tabindex', isActive ? '0' : '-1');
      tab.addEventListener('click', function () {
        activate(tab);
      });
      tab.addEventListener('keydown', function (e) {
        var nextIdx = -1;
        if (e.key === 'ArrowRight') {
          nextIdx = (idx + 1) % tabs.length;
        } else if (e.key === 'ArrowLeft') {
          nextIdx = (idx - 1 + tabs.length) % tabs.length;
        } else if (e.key === 'Home') {
          nextIdx = 0;
        } else if (e.key === 'End') {
          nextIdx = tabs.length - 1;
        }
        if (nextIdx !== -1) {
          e.preventDefault();
          tabs[nextIdx].focus();
          activate(tabs[nextIdx]);
        }
      });
    });
  }

  function fallbackCopy(text, done) {
    var ta = document.createElement('textarea');
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand('copy');
    document.body.removeChild(ta);
    done();
  }

  /* Hero terminal tabs + one-click copy */
  var termTabs = document.querySelectorAll('.hero-tab[data-tab]');
  var termPanels = document.querySelectorAll('.hero-panel[data-panel]');
  var heroCopyBtn = document.querySelector('.hero-copy-btn');

  if (termTabs.length && termPanels.length) {
    wireTabs(termTabs, function (tab) {
      var target = tab.getAttribute('data-tab');
      termPanels.forEach(function (p) {
        p.hidden = p.getAttribute('data-panel') !== target;
      });
    });
  }

  if (heroCopyBtn) {
    heroCopyBtn.addEventListener('click', function () {
      var activePanel = document.querySelector('.hero-panel:not([hidden]) code');
      if (!activePanel) return;
      var text = activePanel.textContent.trim();
      var done = function () {
        heroCopyBtn.textContent = 'copied';
        setTimeout(function () { heroCopyBtn.textContent = 'copy'; }, 1600);
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

  /* 10-second browser rubric sandbox (matches src/presets.rs + src/filter.rs) */
  var PRESETS = {
    'reasoning-math': {
      min_scores: { reasoning_depth: 2.0 },
      reject_nouls: { has_circular_reasoning: 0.70 },
      min_confidence: 0.5,
      sample: [
        '{"id":"math-1","text":"Let x = 3. Then 2x + 4 = 2(3) + 4 = 10. Therefore x = 3 satisfies the equation."}',
        '{"id":"math-2","text":"This is true because it must be true, which proves the premise by definition."}',
        '{"id":"math-3","text":"Using key sk-proj-99887766554433221100aabbccddeeff to query the solver."}'
      ].join('\n')
    },
    'anti-sycophancy': {
      min_scores: {},
      reject_nouls: { is_sycophantic: 0.65, has_ai_disclaimer: 0.80 },
      min_confidence: 0.5,
      sample: [
        '{"id":"chat-1","text":"Binary search runs in O(log n) time on sorted arrays by halving the search interval each step."}',
        '{"id":"chat-2","text":"Great question! As an AI language model, I am thrilled to help you with that!"}',
        '{"id":"chat-3","text":"Export ghp_1234567890abcdef1234567890abcdef1234 before running the script."}'
      ].join('\n')
    },
    'code-correctness': {
      min_scores: { code_quality: 2.0 },
      reject_nouls: { has_stub_placeholders: 0.75 },
      min_confidence: 0.5,
      sample: [
        '{"id":"code-1","text":"fn add(a: i32, b: i32) -> i32 { a.saturating_add(b) }"}',
        '{"id":"code-2","text":"def process(x):\\n    # TODO: implement later\\n    pass"}',
        '{"id":"code-3","text":"const aws = \\"AKIAIOSFODNN7EXAMPLE\\";"}'
      ].join('\n')
    }
  };

  function secretScan(text) {
    if (/\bsk-[A-Za-z0-9_-]{16,}/.test(text)) {
      return 'secret pattern sk- token detected';
    }
    var lower = text.toLowerCase();
    var pats = ['ghp_', 'gho_', 'aws_access_key', 'typesafe_api_key'];
    for (var i = 0; i < pats.length; i++) {
      if (lower.indexOf(pats[i]) !== -1) {
        return 'secret pattern ' + pats[i] + ' detected';
      }
    }
    if (text.indexOf('AKIA') !== -1) {
      return 'possible AWS key';
    }
    return null;
  }

  function evaluateRow(text, presetName) {
    var preset = PRESETS[presetName];
    var secret = secretScan(text);
    if (secret) {
      return {
        passed: false,
        reasons: ['Secret scan: ' + secret],
        scores: {},
        nouls: {},
        confidence: 1.0
      };
    }

    var trimmed = text.trim();
    if (!trimmed) {
      return {
        passed: false,
        reasons: ['Pre-filter: empty record'],
        scores: {},
        nouls: {},
        confidence: 1.0
      };
    }

    var lower = trimmed.toLowerCase();
    var scores = {};
    var nouls = {};
    var reasons = [];

    if (presetName === 'reasoning-math') {
      var circular = /because it must be true|proves the premise|by definition/.test(lower) ? 0.85 : 0.12;
      var depth = trimmed.length > 60 && /=|therefore|step/.test(lower) ? 3.2 : 1.4;
      nouls.has_circular_reasoning = circular;
      scores.reasoning_depth = depth;
      if (circular >= preset.reject_nouls.has_circular_reasoning) {
        reasons.push('has_circular_reasoning (' + circular.toFixed(2) + ' >= ' + preset.reject_nouls.has_circular_reasoning.toFixed(2) + ')');
      }
      if (depth < preset.min_scores.reasoning_depth) {
        reasons.push('reasoning_depth (' + depth.toFixed(1) + ' < ' + preset.min_scores.reasoning_depth.toFixed(1) + ')');
      }
    } else if (presetName === 'anti-sycophancy') {
      var syc = /great question|thrilled to help|absolutely right/.test(lower) ? 0.82 : 0.08;
      var disc = /as an ai language model|as an ai/.test(lower) ? 0.91 : 0.04;
      nouls.is_sycophantic = syc;
      nouls.has_ai_disclaimer = disc;
      if (syc >= preset.reject_nouls.is_sycophantic) {
        reasons.push('is_sycophantic (' + syc.toFixed(2) + ' >= ' + preset.reject_nouls.is_sycophantic.toFixed(2) + ')');
      }
      if (disc >= preset.reject_nouls.has_ai_disclaimer) {
        reasons.push('has_ai_disclaimer (' + disc.toFixed(2) + ' >= ' + preset.reject_nouls.has_ai_disclaimer.toFixed(2) + ')');
      }
    } else if (presetName === 'code-correctness') {
      var stub = /todo|implement later|pass\b|notimplemented/.test(lower) ? 0.88 : 0.09;
      var qual = stub > 0.5 ? 1.2 : 3.5;
      nouls.has_stub_placeholders = stub;
      scores.code_quality = qual;
      if (stub >= preset.reject_nouls.has_stub_placeholders) {
        reasons.push('has_stub_placeholders (' + stub.toFixed(2) + ' >= ' + preset.reject_nouls.has_stub_placeholders.toFixed(2) + ')');
      }
      if (qual < preset.min_scores.code_quality) {
        reasons.push('code_quality (' + qual.toFixed(1) + ' < ' + preset.min_scores.code_quality.toFixed(1) + ')');
      }
    }

    return {
      passed: reasons.length === 0,
      reasons: reasons,
      scores: scores,
      nouls: nouls,
      confidence: 0.94
    };
  }

  var presetSelect = document.getElementById('sandbox-preset');
  var inputArea = document.getElementById('sandbox-input');
  var cleanOut = document.getElementById('sandbox-clean-out');
  var rejectedOut = document.getElementById('sandbox-rejected-out');
  var cleanCount = document.getElementById('sandbox-clean-count');
  var rejectedCount = document.getElementById('sandbox-rejected-count');
  var secretBadge = document.getElementById('sandbox-secret-badge');
  var outTabs = document.querySelectorAll('.sandbox-out-tab');

  if (!presetSelect || !inputArea || !cleanOut || !rejectedOut) return;

  function runSandbox() {
    var presetName = presetSelect.value;
    var lines = inputArea.value.split('\n').filter(function (l) { return l.trim().length > 0; });
    var cleanLines = [];
    var rejectedLines = [];
    var secretHits = 0;

    lines.forEach(function (line) {
      var text = line;
      var parsed = null;
      try {
        parsed = JSON.parse(line);
        if (parsed && typeof parsed.text === 'string') {
          text = parsed.text;
        }
      } catch (_) {}

      var verdict = evaluateRow(text, presetName);
      if (verdict.reasons.some(function (r) { return r.indexOf('Secret scan:') === 0; })) {
        secretHits++;
      }

      var record = (typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed))
        ? Object.assign({}, parsed)
        : { text: text };
      record._verdict = verdict;

      if (verdict.passed) {
        cleanLines.push(JSON.stringify(record));
      } else {
        rejectedLines.push(JSON.stringify(record));
      }
    });

    cleanOut.textContent = cleanLines.length ? cleanLines.join('\n') : '// 0 clean rows';
    rejectedOut.textContent = rejectedLines.length ? rejectedLines.join('\n') : '// 0 rejected rows';
    if (cleanCount) cleanCount.textContent = String(cleanLines.length);
    if (rejectedCount) rejectedCount.textContent = String(rejectedLines.length);
    if (secretBadge) {
      secretBadge.textContent = secretHits > 0
        ? 'secret_scan: ' + secretHits + ' blocked'
        : 'secret_scan: clean';
      secretBadge.classList.toggle('is-alert', secretHits > 0);
    }
  }

  var inputTimer = null;

  presetSelect.addEventListener('change', function () {
    clearTimeout(inputTimer);
    inputArea.value = PRESETS[presetSelect.value].sample;
    runSandbox();
  });

  inputArea.addEventListener('input', function () {
    clearTimeout(inputTimer);
    inputTimer = setTimeout(runSandbox, 150);
  });

  var resetBtn = document.getElementById('sandbox-reset');
  if (resetBtn) {
    resetBtn.addEventListener('click', function () {
      clearTimeout(inputTimer);
      inputArea.value = PRESETS[presetSelect.value].sample;
      runSandbox();
    });
  }

  wireTabs(outTabs, function (tab) {
    var target = tab.getAttribute('data-out');
    cleanOut.hidden = target !== 'clean';
    rejectedOut.hidden = target !== 'rejected';
  });

  /* Initialize */
  inputArea.value = PRESETS[presetSelect.value].sample;
  runSandbox();
})();
