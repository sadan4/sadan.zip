// Webpack Module 363195 
//EXTRACED WEPBACK MODULE 363195
0,
function(e, t, n) {
    "use strict";
    n.d(t, {
        A: () => v
    });
    var i = n(17928)
      , r = n(462887)
      , s = n(228366)
      , a = n(775602)
      , o = n(677313)
      , l = n(873298)
      , u = n(284016)
      , c = n(742023)
      , d = n(617617)
      , _ = n(652215)
      , f = n(185928)
      , h = n(661531)
      , p = n(353835)
      , E = n(723702);
    function m(e) {
        if (!__OVERLAY__ && E.isPlatformEmbedded)
            try {
                let t = h.A.colors.BACKGROUND_BASE_LOWEST.resolve({
                    theme: e,
                    saturation: a.A.saturation
                }).hex();
                p.A.setApplicationBackgroundColor(t)
            } catch {}
    }
    let g = 0
      , A = f.qj
      , I = (0,
    o.A)()
      , T = A[I]
      , S = null;
    function N() {
        if (!__OVERLAY__ && null != S)
            return S;
        var e = I
          , t = A;
        if (__OVERLAY__)
            return _.NJ8.DARK;
        let n = f.dP;
        if (a.A.syncForcedColors && "active" === a.A.systemForcedColors && e !== f.Fc.NO_PREFERENCE)
            return e;
        if (c.Ay.useSystemTheme === f.Q_.ON && e !== f.Fc.NO_PREFERENCE)
            return t[e];
        let i = u.A.getAppearanceSettings()?.theme;
        return null != i ? i : n[d.A.settings.appearance?.theme ?? l.Sx.UNSET]
    }
    function y() {
        let e = N();
        return e !== T && (m(T = e),
        !0)
    }
    class C extends i.Ay.PersistedStore {
        static displayName = "ThemeStore";
        static persistKey = "ThemeStore";
        static migrations = [e => {
            let t = e.theme;
            return "amoled" === t && (t = "midnight"),
            {
                ...e,
                theme: t
            }
        }
        , e => e];
        initialize(e) {
            e?.theme != null && (g = 1,
            m(T = e.theme),
            null != e.preferences && (A = e.preferences),
            (0,
            r.M)(T) && (A[f.Fc.DARK] = T)),
            this.waitFor(c.Ay, u.A, d.A, a.A)
        }
        getState() {
            return {
                theme: this.theme,
                preferences: A,
                status: g
            }
        }
        get theme() {
            return T
        }
        get systemTheme() {
            return I
        }
        themePreferenceForSystemTheme(e) {
            return A[e]
        }
    }
    let v = new C(s.h,{
        CACHE_LOADED: y,
        CONNECTION_OPEN: function() {
            return 0 === g && (A = {
                ...A,
                [f.Fc.DARK]: _.NJ8.DARKER
            },
            g = 1),
            y()
        },
        LOGOUT: function(e) {
            return S = null,
            !e.isSwitchingAccount && T !== _.NJ8.DARK && (m(T = _.NJ8.DARK),
            y())
        },
        OVERLAY_INITIALIZE: y,
        SELECTIVELY_SYNCED_USER_SETTINGS_UPDATE: y,
        UNSYNCED_USER_SETTINGS_UPDATE: y,
        USER_SETTINGS_PROTO_UPDATE: y,
        RESET_PREVIEW_CLIENT_THEME: y,
        SYSTEM_THEME_CHANGE: function(e) {
            let {systemTheme: t} = e;
            return I = t,
            y()
        },
        ACCESSIBILITY_DARK_SIDEBAR_TOGGLE: function() {
            return (0,
            r.q)(N())
        },
        UPDATE_THEME_PREFERENCES: function(e) {
            return A = {
                ...A,
                ...e.preferences
            },
            y()
        },
        SET_THEME_OVERRIDE: function(e) {
            return S = e.theme,
            y()
        },
        CLEAR_THEME_OVERRIDE: function() {
            return S = null,
            y()
        },
        REFRESH_THEME: function() {
            return y()
        }
    })
}
