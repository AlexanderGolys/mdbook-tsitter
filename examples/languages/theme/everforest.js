// mdBook gives code editors its light theme for any theme it does not know, so
// switch them to its dark one while Everforest is active.
(function everforestEditors() {
    const theme = 'everforest';
    const darkEditorTheme = 'ace/theme/tomorrow_night';
    const html = document.documentElement;

    function useDarkEditors() {
        if (html.classList.contains(theme) && window.editors) {
            window.editors.forEach(function(editor) {
                editor.setTheme(darkEditorTheme);
            });
        }
    }

    new MutationObserver(useDarkEditors).observe(html, { attributeFilter: ['class'] });
    useDarkEditors();
})();
