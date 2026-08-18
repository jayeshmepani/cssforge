document.addEventListener('DOMContentLoaded', () => {
  'use strict';

  // ── 1. Theme Switcher ──
  const THEME_KEY = 'cssforge-docs-theme';
  const root = document.documentElement;
  const themeBtn = document.getElementById('theme-toggle-btn');

  function applyTheme(theme, persist) {
    root.setAttribute('data-theme', theme);
    root.style.colorScheme = theme;
    if (persist) {
      try { localStorage.setItem(THEME_KEY, theme); } catch (e) {}
    }
  }

  const storedTheme = (() => {
    try { return localStorage.getItem(THEME_KEY); } catch (e) { return null; }
  })();

  if (storedTheme) {
    applyTheme(storedTheme, false);
  } else if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
    applyTheme('dark', false);
  } else {
    applyTheme('light', false);
  }

  if (themeBtn) {
    themeBtn.addEventListener('click', () => {
      const current = root.getAttribute('data-theme') === 'dark' ? 'light' : 'dark';
      applyTheme(current, true);
    });
  }

  // ── 2. Toast System ──
  const toastContainer = document.getElementById('toast-container');
  function showToast(msg) {
    if (!toastContainer) return;
    const toast = document.createElement('div');
    toast.className = 'toast';
    toast.textContent = msg;
    toastContainer.appendChild(toast);
    requestAnimationFrame(() => toast.classList.add('is-visible'));
    setTimeout(() => {
      toast.classList.remove('is-visible');
      setTimeout(() => toast.remove(), 250);
    }, 2000);
  }

  // ── 2.1. Universal Accessible Custom Select Dropdown Controller ──
  function setupCustomSelects() {
    document.querySelectorAll('select.custom-select').forEach(select => {
      if (select.closest('.custom-select-wrapper')) return;

      const wrapper = document.createElement('div');
      wrapper.className = 'custom-select-wrapper';
      select.parentNode.insertBefore(wrapper, select);
      wrapper.appendChild(select);

      const trigger = document.createElement('button');
      trigger.type = 'button';
      trigger.className = 'custom-select-trigger';
      trigger.setAttribute('aria-haspopup', 'listbox');
      trigger.setAttribute('aria-expanded', 'false');

      const label = document.createElement('span');
      label.className = 'trigger-label';
      label.textContent = select.options[select.selectedIndex]?.text || '';

      const chevron = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
      chevron.setAttribute('class', 'trigger-chevron');
      chevron.setAttribute('viewBox', '0 0 20 20');
      chevron.setAttribute('fill', 'currentColor');
      chevron.innerHTML = '<path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" clip-rule="evenodd"/>';

      trigger.appendChild(label);
      trigger.appendChild(chevron);
      wrapper.appendChild(trigger);

      const dropdown = document.createElement('div');
      dropdown.className = 'custom-select-dropdown';
      dropdown.setAttribute('role', 'listbox');

      let focusedIndex = select.selectedIndex >= 0 ? select.selectedIndex : 0;

      function buildOptions() {
        dropdown.innerHTML = '';
        Array.from(select.options).forEach((opt, idx) => {
          const item = document.createElement('div');
          item.className = 'custom-select-option' + (opt.selected ? ' is-selected' : '');
          item.setAttribute('role', 'option');
          item.setAttribute('aria-selected', opt.selected ? 'true' : 'false');
          item.dataset.value = opt.value;
          item.tabIndex = -1;

          const textSpan = document.createElement('span');
          textSpan.className = 'option-text';
          textSpan.textContent = opt.text;

          const checkSpan = document.createElement('span');
          checkSpan.className = 'option-check';
          checkSpan.textContent = '✓';

          item.appendChild(textSpan);
          item.appendChild(checkSpan);

          item.addEventListener('click', (e) => {
            e.stopPropagation();
            selectOption(idx);
          });

          item.addEventListener('mouseenter', () => {
            setFocusedOption(idx);
          });

          dropdown.appendChild(item);
        });
      }

      function selectOption(idx) {
        select.selectedIndex = idx;
        focusedIndex = idx;
        label.textContent = select.options[idx]?.text || '';
        closeAllSelects();
        select.dispatchEvent(new Event('change', { bubbles: true }));
        updateSelectedState();
        trigger.focus();
      }

      function setFocusedOption(idx) {
        focusedIndex = idx;
        dropdown.querySelectorAll('.custom-select-option').forEach((item, i) => {
          item.classList.toggle('is-focused', i === idx);
        });
      }

      function updateSelectedState() {
        label.textContent = select.options[select.selectedIndex]?.text || '';
        dropdown.querySelectorAll('.custom-select-option').forEach((item, idx) => {
          const isSelected = idx === select.selectedIndex;
          item.classList.toggle('is-selected', isSelected);
          item.setAttribute('aria-selected', isSelected ? 'true' : 'false');
        });
      }

      buildOptions();
      wrapper.appendChild(dropdown);

      function openDropdown() {
        closeAllSelects();
        trigger.classList.add('is-open');
        trigger.setAttribute('aria-expanded', 'true');
        dropdown.classList.add('is-open');
        setFocusedOption(select.selectedIndex >= 0 ? select.selectedIndex : 0);
      }

      function closeDropdown() {
        trigger.classList.remove('is-open');
        trigger.setAttribute('aria-expanded', 'false');
        dropdown.classList.remove('is-open');
      }

      trigger.addEventListener('click', (e) => {
        e.stopPropagation();
        const isOpen = trigger.classList.contains('is-open');
        if (isOpen) {
          closeDropdown();
        } else {
          openDropdown();
        }
      });

      // Keyboard Navigation on Trigger
      trigger.addEventListener('keydown', (e) => {
        const isOpen = trigger.classList.contains('is-open');
        const count = select.options.length;

        if (e.key === 'ArrowDown' || e.key === 'Down') {
          e.preventDefault();
          if (!isOpen) {
            openDropdown();
          } else {
            focusedIndex = (focusedIndex + 1) % count;
            setFocusedOption(focusedIndex);
          }
        } else if (e.key === 'ArrowUp' || e.key === 'Up') {
          e.preventDefault();
          if (!isOpen) {
            openDropdown();
          } else {
            focusedIndex = (focusedIndex - 1 + count) % count;
            setFocusedOption(focusedIndex);
          }
        } else if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          if (isOpen) {
            selectOption(focusedIndex);
          } else {
            openDropdown();
          }
        } else if (e.key === 'Escape' || e.key === 'Esc') {
          if (isOpen) {
            e.preventDefault();
            closeDropdown();
          }
        } else if (e.key === 'Tab') {
          closeDropdown();
        }
      });

      select.addEventListener('change', () => {
        updateSelectedState();
      });
    });
  }

  function closeAllSelects() {
    document.querySelectorAll('.custom-select-trigger.is-open').forEach(tr => {
      tr.classList.remove('is-open');
      tr.setAttribute('aria-expanded', 'false');
    });
    document.querySelectorAll('.custom-select-dropdown.is-open').forEach(dd => {
      dd.classList.remove('is-open');
    });
  }

  document.addEventListener('click', (e) => {
    if (!e.target.closest('.custom-select-wrapper')) {
      closeAllSelects();
    }
  });

  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      closeAllSelects();
    }
  });

  setupCustomSelects();

  // ── 3. Copy Code Snippets ──
  document.querySelectorAll('pre').forEach(pre => {
    const btn = document.createElement('button');
    btn.className = 'copy-code-btn';
    btn.textContent = 'Copy';
    pre.appendChild(btn);

    btn.addEventListener('click', async () => {
      const code = pre.querySelector('code')?.innerText || pre.innerText.replace('Copy', '').trim();
      try {
        await navigator.clipboard.writeText(code);
        showToast('✓ Code snippet copied to clipboard');
        btn.textContent = 'Copied!';
        setTimeout(() => { btn.textContent = 'Copy'; }, 2000);
      } catch (err) {
        showToast('Failed to copy');
      }
    });
  });

  // ── 4. Mobile Drawer Navigation ──
  const menuBtn = document.getElementById('menu-toggle-btn');
  const sidebar = document.getElementById('sidebar');
  const backdrop = document.getElementById('drawer-backdrop');

  function toggleDrawer(open) {
    if (!sidebar || !backdrop) return;
    sidebar.classList.toggle('is-open', open);
    backdrop.classList.toggle('is-active', open);
  }

  if (menuBtn) {
    menuBtn.addEventListener('click', () => toggleDrawer(!sidebar.classList.contains('is-open')));
  }
  if (backdrop) {
    backdrop.addEventListener('click', () => toggleDrawer(false));
  }
  if (sidebar) {
    sidebar.addEventListener('click', (e) => {
      if (e.target.closest('a') && window.innerWidth <= 1024) {
        toggleDrawer(false);
      }
    });
  }

  // ── 5. OS Installation Tabs ──
  document.querySelectorAll('.tab-container').forEach(container => {
    const buttons = container.querySelectorAll('.tab-btn');
    const panes = container.querySelectorAll('.tab-pane');

    buttons.forEach(btn => {
      btn.addEventListener('click', () => {
        const target = btn.dataset.tab;
        buttons.forEach(b => b.classList.remove('active'));
        panes.forEach(p => p.classList.remove('active'));

        btn.classList.add('active');
        const activePane = container.querySelector(`.tab-pane[data-tab="${target}"]`);
        if (activePane) activePane.classList.add('active');
      });
    });
  });

  // ── 6. Rule Catalog Search & Category Filter ──
  const ruleSearchInput = document.getElementById('rule-search-input');
  const ruleCategorySelect = document.getElementById('rule-category-select');
  const categoryPills = document.querySelectorAll('.pill-btn');
  const ruleCards = document.querySelectorAll('.rule-card');
  let activeCategory = 'all';

  function filterRules() {
    const query = ruleSearchInput ? ruleSearchInput.value.toLowerCase().trim() : '';

    ruleCards.forEach(card => {
      const category = card.dataset.category || 'all';
      const text = card.textContent.toLowerCase();
      const matchesCategory = activeCategory === 'all' || category === activeCategory;
      const matchesQuery = !query || text.includes(query);

      card.style.display = matchesCategory && matchesQuery ? 'block' : 'none';
    });
  }

  if (ruleSearchInput) {
    ruleSearchInput.addEventListener('input', filterRules);
  }

  if (ruleCategorySelect) {
    ruleCategorySelect.addEventListener('change', (e) => {
      activeCategory = e.target.value;
      categoryPills.forEach(p => {
        p.classList.toggle('active', p.dataset.category === activeCategory);
      });
      filterRules();
    });
  }

  categoryPills.forEach(pill => {
    pill.addEventListener('click', () => {
      categoryPills.forEach(p => p.classList.remove('active'));
      pill.classList.add('active');
      activeCategory = pill.dataset.category;
      if (ruleCategorySelect) {
        ruleCategorySelect.value = activeCategory;
      }
      filterRules();
    });
  });

  // ── 7. Sidebar Topic Search ──
  const navSearch = document.getElementById('nav-search-input');
  const navLinks = document.querySelectorAll('.nav-list a');

  if (navSearch) {
    navSearch.addEventListener('input', (e) => {
      const q = e.target.value.toLowerCase().trim();
      navLinks.forEach(link => {
        const text = link.textContent.toLowerCase();
        link.parentElement.style.display = text.includes(q) ? 'block' : 'none';
      });
    });
  }

  // ── 8. ScrollSpy for Sidebar Active Anchor ──
  const sections = Array.from(document.querySelectorAll('section[id]'));
  window.addEventListener('scroll', () => {
    let currentId = '';
    const scrollPos = window.scrollY + 100;
    sections.forEach(section => {
      if (section.offsetTop <= scrollPos) {
        currentId = section.getAttribute('id');
      }
    });

    navLinks.forEach(link => {
      if (link.getAttribute('href') === `#${currentId}`) {
        link.classList.add('active');
      } else {
        link.classList.remove('active');
      }
    });
  });
});
