// Webpack Module 82716
//EXTRACTED WEBPACK MODULE 82716
0,
function(e, t, n) {
    n.d(t, {
        z: () => f
    });
    var l = n(627968)
      , a = n(64700)
      , r = n(408278)
      , i = n(27232)
      , s = n(505930)
      , u = n(990078)
      , c = n(609174)
      , o = n(430795)
      , d = n(16590)
      , p = n(375708);
    function f() {
        let e = (0,
        c.Y_)()
          , t = a.useCallback(t => {
            t.stopPropagation(),
            t.preventDefault(),
            (0,
            o.XK)(e)
        }
        , [e]);
        return (0,
        l.jsx)(u.m, {
            text: p.intl.string(e.isFavorite ? d.default.IZsalP : d.default.ihBfyA),
            position: "top",
            children: (0,
            l.jsx)(r.K, {
                onClick: t,
                icon: e.isFavorite ? i.G : s.y,
                "aria-label": p.intl.string(p.t.k8fFjp),
                variant: "overlay-secondary",
                size: "sm"
            })
        }, `${e.id}:favorite:${e.isFavorite}`)
    }
}
