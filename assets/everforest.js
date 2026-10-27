// Register before mdBook initialises the theme picker. Chapters, including
// the print page, can each contain this script, so registration is idempotent.
(function registerEverforest() {
    const theme = 'everforest';
    const buttonId = 'mdbook-theme-' + theme;
    const menu = document.getElementById('mdbook-theme-list');
    if (menu && !document.getElementById(buttonId)) {
        const item = document.createElement('li');
        item.setAttribute('role', 'none');
        const button = document.createElement('button');
        button.id = buttonId;
        button.className = 'theme';
        button.setAttribute('role', 'menuitem');
        button.textContent = 'Everforest';
        item.appendChild(button);
        menu.appendChild(item);
    }

    const html = document.documentElement;
    if (html.hasAttribute('data-everforest-registered')) {
        return;
    }
    html.setAttribute('data-everforest-registered', '');

    // mdBook gives unknown themes light editors; use its bundled dark theme.
    function useDarkEditors() {
        if (html.classList.contains(theme) && window.editors) {
            window.editors.forEach(function(editor) {
                editor.setTheme('ace/theme/tomorrow_night');
            });
        }
    }

    new MutationObserver(useDarkEditors).observe(html, { attributeFilter: ['class'] });
    window.addEventListener('load', useDarkEditors);
    useDarkEditors();
})();
