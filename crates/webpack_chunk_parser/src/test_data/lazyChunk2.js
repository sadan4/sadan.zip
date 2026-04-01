"use strict";
(this.webpackChunkdiscord_app = this.webpackChunkdiscord_app || []).push([["52694"], {
    348858(e, t, r) {
        r.d(t, {
            I: () => u
        });
        var n = r(627968)
          , a = r(64700)
          , c = r(744682);
        let l = {
            deafen: {
                name: "deafen",
                start: 0,
                duration: 70
            },
            undeafen: {
                name: "undeafen",
                start: 110,
                duration: 70
            },
            hover_undeafened: {
                name: "hover_undeafened",
                start: 200,
                duration: 70
            },
            hover_deafened: {
                name: "hover_deafened",
                start: 300,
                duration: 70
            }
        }
          , u = e => {
            let t = a.useRef(null)
              , u = a.useRef(e);
            u.current = e;
            let s = a.useMemo( () => () => {
                null != t.current && t.current.play(e)
            }
            , [e])
              , i = a.useCallback( () => {
                if (null == t.current)
                    return;
                let r = "deafen" === e ? "hover_undeafened" : "hover_deafened";
                t.current.play(r)
            }
            , [e])
              , o = a.useCallback( () => {
                if (null == t.current)
                    return;
                let r = "deafen" === e ? "hover_undeafened" : "hover_deafened";
                t.current.stopIfPlaying(r)
            }
            , [e])
              , d = a.useCallback(e => {
                var a, s;
                return (0,
                n.jsx)(c.P, (a = function(e) {
                    for (var t = 1; t < arguments.length; t++) {
                        var r = null != arguments[t] ? arguments[t] : {}
                          , n = Object.keys(r);
                        "function" == typeof Object.getOwnPropertySymbols && (n = n.concat(Object.getOwnPropertySymbols(r).filter(function(e) {
                            return Object.getOwnPropertyDescriptor(r, e).enumerable
                        }))),
                        n.forEach(function(t) {
                            var n;
                            n = r[t],
                            t in e ? Object.defineProperty(e, t, {
                                value: n,
                                enumerable: !0,
                                configurable: !0,
                                writable: !0
                            }) : e[t] = n
                        })
                    }
                    return e
                }({}, e),
                s = s = {
                    src: () => r.e("93768").then(r.t.bind(r, 894619, 19)),
                    ref: t,
                    initialAnimation: u.current,
                    markers: l
                },
                Object.getOwnPropertyDescriptors ? Object.defineProperties(a, Object.getOwnPropertyDescriptors(s)) : (function(e, t) {
                    var r = Object.keys(e);
                    if (Object.getOwnPropertySymbols) {
                        var n = Object.getOwnPropertySymbols(e);
                        r.push.apply(r, n)
                    }
                    return r
                }
                )(Object(s)).forEach(function(e) {
                    Object.defineProperty(a, e, Object.getOwnPropertyDescriptor(s, e))
                }),
                a))
            }
            , []);
            return {
                events: {
                    onClick: s,
                    onMouseEnter: i,
                    onMouseLeave: o
                },
                play: s,
                getDuration: a.useCallback( () => {
                    var e;
                    return null == (e = t.current) ? void 0 : e.getDuration()
                }
                , []),
                getCurrentFrame: a.useCallback( () => {
                    var e, r;
                    return null != (e = null == (r = t.current) ? void 0 : r.getCurrentFrame()) ? e : null
                }
                , []),
                Component: d
            }
        }
    },
    617354(e, t, r) {
        r.d(t, {
            A: () => a
        });
        var n = r(985018);
        function a(e, t, r) {
            return r ? n.intl.string(n.t["2Ne/Y1"]) : t ? n.intl.string(n.t.QZ7WSS) : e ? n.intl.string(n.t["2US872"]) : n.intl.string(n.t.wjcRFX)
        }
    },
    18235(e, t, r) {
        r.d(t, {
            A: () => l
        });
        var n = r(827343)
          , a = r(579872)
          , c = r(985018);
        function l(e, t) {
            e ? a.A.show({
                title: c.intl.string(c.t.QZ7WSS),
                body: c.intl.string(c.t.Tl9JpL)
            }) : n.A.toggleSelfDeaf({
                location: t
            })
        }
    },
    302223(e, t, r) {
        r.d(t, {
            A: () => s
        });
        var n = r(627968);
        r(64700);
        var a = r(503698)
          , c = r.n(a)
          , l = r(51183)
          , u = r(802455);
        function s(e) {
            let {activity: t, className: r, emojiClassName: a, textClassName: s, placeholderText: i, soloEmojiClassName: o, animate: d=!0, hideTooltip: f=!1, hideEmoji: m=!1, children: x} = e;
            if (null == t)
                return null;
            let {emoji: h} = t
              , b = null != t.state && "" !== t.state ? t.state : i;
            return (0,
            n.jsxs)("div", {
                className: c()(u.__invalid_container, r),
                children: [m || null == h ? null : (0,
                n.jsx)(l.A, {
                    emoji: h,
                    className: c()(u.Z, a, null != o ? {
                        [o]: null == b || "" === b
                    } : null),
                    animate: d,
                    hideTooltip: f
                }), null != b && b.length > 0 ? (0,
                n.jsx)("span", {
                    className: s,
                    children: b
                }) : null, x]
            })
        }
    },
    706712(e, t, r) {
        r.d(t, {
            Ay: () => O,
            DQ: () => w,
            Dj: () => k,
            F5: () => j,
            Jc: () => b,
            L6: () => x,
            ZX: () => S,
            km: () => v
        }),
        r(896048);
        var n = r(627968)
          , a = r(64700)
          , c = r(503698)
          , l = r.n(c)
          , u = r(311907)
          , s = r(990078)
          , i = r(397927)
          , o = r(964486)
          , d = r(142120)
          , f = r(132262)
          , m = r(661251);
        let x = 1e3 / 60
          , h = 1e3 / 30
          , b = 5e3
          , g = 1e3 / 60 * 3
          , p = Math.ceil(3e3 / (1e3 / 60));
        function v(e, t) {
            let r = arguments.length > 2 && void 0 !== arguments[2] ? arguments[2] : window
              , n = a.useRef(null)
              , c = a.useRef(null)
              , l = a.useRef(null)
              , u = a.useRef(null != r ? r : window);
            a.useEffect( () => {
                u.current = null != r ? r : window
            }
            , [r]);
            let s = a.useCallback( () => {
                null != n.current && u.current.clearInterval(n.current),
                null != c.current && u.current.cancelIdleCallback(c.current),
                null != l.current && u.current.cancelAnimationFrame(l.current)
            }
            , [])
              , i = a.useCallback( () => {
                n.current = u.current.setTimeout( () => {
                    c.current = u.current.requestIdleCallback(e),
                    l.current = u.current.requestAnimationFrame( () => {
                        t(),
                        i()
                    }
                    )
                }
                , 12)
            }
            , [e, t]);
            return [a.useCallback( () => {
                s(),
                i()
            }
            , [s, i]), s]
        }
        function j(e) {
            let t = a.useRef(Array(p).fill(0))
              , r = a.useRef(performance.now())
              , n = a.useRef(0)
              , c = a.useRef(0)
              , l = a.useRef(0)
              , u = e.dispatcher.getIsSchedulerBackgrounded()
              , s = a.useRef(u);
            s.current = u;
            let i = a.useRef(u ? performance.now() : 0);
            return a.useEffect( () => {
                e.dispatcher.getIsSchedulerBackgrounded() && (i.current = performance.now())
            }
            ),
            [a.useCallback(function() {
                let e = performance.now()
                  , a = e - r.current;
                r.current = e,
                s.current || (n.current -= t.current[l.current],
                t.current[l.current] = a,
                n.current += a,
                c.current < p && (c.current += 1),
                l.current = (l.current + 1) % p)
            }, []), (e, t) => {
                var r;
                let a = null != (r = c.current) ? r : 1;
                return Math.abs(e * t - n.current / a * a) / t
            }
            , () => {
                n.current = 0,
                c.current = 0,
                t.current.fill(0),
                r.current = performance.now(),
                l.current = 0
            }
            ]
        }
        function k(e, t) {
            let r = arguments.length > 2 && void 0 !== arguments[2] && arguments[2]
              , n = a.useRef(Array(p).fill(0))
              , c = a.useRef(performance.now())
              , l = a.useRef(0)
              , u = a.useRef(0)
              , s = a.useRef(0)
              , i = a.useRef(0)
              , o = a.useRef(0)
              , d = a.useRef(0)
              , f = a.useCallback( () => {
                n.current.fill(0),
                l.current = 0,
                u.current = 0,
                i.current = 0,
                o.current = 0,
                c.current = performance.now(),
                s.current = 0
            }
            , [])
              , m = a.useCallback(function() {
                let a = performance.now()
                  , f = a - c.current;
                if (c.current = a,
                t.current && !r)
                    return;
                if (u.current -= n.current[o.current],
                n.current[o.current] = f,
                u.current += f,
                i.current < p && (i.current += 1),
                o.current = (o.current + 1) % p,
                f > g) {
                    let t = 0 === i.current ? x : u.current / i.current
                      , r = Math.min(2 * x, t)
                      , n = Math.floor(f / (e ? r : x));
                    n > 0 && (d.current = performance.now()),
                    l.current += n
                }
                let m = 0 === i.current ? x : u.current / i.current;
                s.current += f / m
            }, [e, t, r])
              , h = 0 === i.current ? 0 : u.current / i.current;
            return {
                currentFPS: 0 === h ? 0 : x / h * 60,
                averageFrameTime: h,
                timeSinceLastDrop: (performance.now() - d.current) / 1e3,
                droppedFramesRef: l,
                bufferFramecountRef: i,
                renderedFrameCount: s,
                frameCheckerEffect: m,
                onResetFrameData: f
            }
        }
        function w(e) {
            let t = e.dispatcher.getIsSchedulerBackgrounded()
              , r = a.useRef(t);
            r.current = t;
            let n = a.useRef(t ? performance.now() : 0);
            return a.useEffect( () => {
                e.dispatcher.getIsSchedulerBackgrounded() && (n.current = performance.now())
            }
            ),
            [r, n]
        }
        function y(e) {
            let {socket: t, isAverageFrameTime: r} = e
              , [c,l] = w(t)
              , {currentFPS: u, averageFrameTime: d, timeSinceLastDrop: m, onResetFrameData: h, droppedFramesRef: g, renderedFrameCount: p, bufferFramecountRef: y, frameCheckerEffect: T} = k(r, c)
              , [R,C,S] = j(t)
              , [O,E] = v(R, T)
              , F = performance.now() - l.current < b
              , I = C(d, y.current);
            (0,
            o.Ay)( () => (O(),
            () => {
                E()
            }
            ));
            let A = a.useCallback( () => {
                h(),
                S(),
                O()
            }
            , [h, S, O]);
            return (0,
            n.jsxs)("div", {
                className: f.st,
                children: [(0,
                n.jsxs)(i.Text, {
                    variant: "text-md/normal",
                    color: "text-muted",
                    children: ["FPS (~3sec):", " ", (0,
                    n.jsx)(i.Text, {
                        tag: "span",
                        variant: "text-md/bold",
                        color: u < 30 ? "text-feedback-critical" : u < 45 ? "text-feedback-warning" : "text-strong",
                        children: u.toFixed(2)
                    })]
                }), (0,
                n.jsxs)(i.Text, {
                    variant: "text-md/normal",
                    color: "text-muted",
                    children: ["Dropped Frames:", " ", (0,
                    n.jsx)(i.Text, {
                        tag: "span",
                        variant: "text-md/bold",
                        color: m < 2 ? "text-feedback-critical" : m < 5 ? "text-feedback-warning" : "text-strong",
                        children: g.current
                    }), (0,
                    n.jsxs)(i.Text, {
                        tag: "span",
                        variant: "text-sm/normal",
                        color: "text-muted",
                        className: f.af,
                        children: ["(Dropped: ", (g.current / p.current * 100).toFixed(4), "%)"]
                    }), F && (0,
                    n.jsx)(s.m, {
                        position: "left",
                        text: "We don't track frames while the app is in the background, because requestAnimationFrame doesn't fire in the background",
                        asContainer: !0,
                        children: (0,
                        n.jsx)(i.Text, {
                            tag: "span",
                            variant: "text-xs/bold",
                            color: "text-feedback-critical",
                            className: f.af,
                            children: "(Backgrounded)"
                        })
                    })]
                }), (0,
                n.jsxs)(i.Text, {
                    variant: "text-md/normal",
                    color: "text-muted",
                    children: ["Rendered Frames:", " ", (0,
                    n.jsx)(i.Text, {
                        tag: "span",
                        variant: "text-md/semibold",
                        color: "text-subtle",
                        children: p.current.toFixed(0)
                    })]
                }), (0,
                n.jsxs)(i.Text, {
                    variant: "text-md/normal",
                    color: "text-muted",
                    children: ["Frame Times (~3sec):", " ", (0,
                    n.jsxs)(i.Text, {
                        tag: "span",
                        variant: "text-md/semibold",
                        color: d > 1.1 * x ? "text-feedback-warning" : "text-subtle",
                        children: [d.toFixed(2), "ms"]
                    })]
                }), (0,
                n.jsx)(s.m, {
                    position: "left",
                    text: "The average amount of 'lag' between us rendering a frame and being able to process background tasks. Values constantly above 1-2ms means our main thread is being burried by work and is taking all of its time in animation frames, most likely producing user interaciton blocking jank. (This doesn't work when the app is backgrounded though)",
                    asContainer: !0,
                    children: (0,
                    n.jsxs)(i.Text, {
                        variant: "text-md/normal",
                        color: "text-muted",
                        children: ["Idle Frame Delta (~3sec):", " ", (0,
                        n.jsxs)(i.Text, {
                            tag: "span",
                            variant: "text-md/semibold",
                            color: I > 1 ? "text-feedback-critical" : "text-subtle",
                            children: [I.toFixed(2), "ms"]
                        }), F && (0,
                        n.jsx)(s.m, {
                            position: "left",
                            text: "We don't track frames while the app is in the background, because requestAnimationFrame doesn't fire in the background",
                            asContainer: !0,
                            children: (0,
                            n.jsx)(i.Text, {
                                tag: "span",
                                variant: "text-xs/bold",
                                color: "text-feedback-critical",
                                className: f.af,
                                children: "(Backgrounded)"
                            })
                        })]
                    })
                }), (0,
                n.jsx)("div", {
                    className: f.m8,
                    children: (0,
                    n.jsx)(i.Button, {
                        variant: "primary",
                        size: "sm",
                        text: "Reset Frame Data",
                        onClick: A
                    })
                })]
            })
        }
        function T(e) {
            let {socket: t, isAverageFrameTime: r, onToggleAverageFrameTime: c} = e
              , [l,u] = a.useState(t.dispatcher.getIsRequestIdleCallbackEnabled())
              , o = a.useRef(null);
            return a.useEffect( () => (o.current = setInterval( () => {
                u(t.dispatcher.getIsRequestIdleCallbackEnabled())
            }
            , h),
            () => {
                null != o.current && clearInterval(o.current)
            }
            ), [t.dispatcher]),
            (0,
            n.jsxs)("div", {
                className: f.st,
                children: [(0,
                n.jsx)(s.m, {
                    position: "left",
                    text: "Instead of using 60fps to calculate the number of dropped frames, we use the average framerate to more accurately determine the number of actual dropped frames. Turn this off when benchmarking to get better comparsion between two different runtimes, where higher FPS might result in a higher dropped frame count.",
                    asContainer: !0,
                    children: (0,
                    n.jsx)(i.Checkbox, {
                        label: "Use Average Frame Time",
                        checked: r,
                        onChange: () => c(!r)
                    })
                }), (0,
                n.jsx)(i.Checkbox, {
                    label: "Enable New Dispatch Scheduler (requestIdleCallback)",
                    checked: l,
                    onChange: () => {
                        var e;
                        return e = !l,
                        void (t.dispatcher.toggleRequestIdleCallback(e),
                        u(e))
                    }
                })]
            })
        }
        function R(e) {
            let {socket: t} = e
              , r = t.dispatcher.getDispatchTimings()
              , [c,u] = a.useState(!1);
            return (0,
            n.jsxs)("div", {
                className: f.st,
                children: [(0,
                n.jsx)("div", {
                    className: l()(c && f.Mq),
                    children: (0,
                    n.jsx)(i.Checkbox, {
                        label: "Show Dispatch Timings",
                        checked: c,
                        onChange: () => u(e => !e)
                    })
                }), c ? (0,
                n.jsxs)(n.Fragment, {
                    children: [(0,
                    n.jsx)(i.Text, {
                        variant: "text-md/medium",
                        color: "text-muted",
                        children: "Gateway Dispatch Timings:"
                    }), (0,
                    n.jsx)("table", {
                        cellPadding: 4,
                        children: Object.entries(r).map(e => {
                            let[t,[r,a]] = e;
                            return (0,
                            n.jsxs)("tr", {
                                children: [(0,
                                n.jsx)("td", {
                                    children: (0,
                                    n.jsx)(i.Text, {
                                        variant: "text-xs/normal",
                                        color: "text-default",
                                        children: t
                                    })
                                }), (0,
                                n.jsx)("td", {
                                    children: (0,
                                    n.jsxs)(i.Text, {
                                        tag: "span",
                                        variant: "text-xs/bold",
                                        color: "text-default",
                                        children: [r.toFixed(2), "ms"]
                                    })
                                }), (0,
                                n.jsx)("td", {
                                    children: (0,
                                    n.jsxs)(i.Text, {
                                        tag: "span",
                                        variant: "text-xs/normal",
                                        color: "text-muted",
                                        children: ["(count: ", a, ")"]
                                    })
                                })]
                            }, t)
                        }
                        )
                    })]
                }) : null]
            })
        }
        function C(e) {
            let {socket: t} = e
              , r = t.dispatcher.getSchedulerTelemetry()
              , [c,u] = a.useState(r.isTelemetryEnabled)
              , [s,o] = a.useState(r.isTelemetryEnabled)
              , d = e => {
                o(e),
                r.toggleTelemetry(e)
            }
            ;
            return (0,
            n.jsxs)("div", {
                className: f.st,
                children: [(0,
                n.jsx)(i.Checkbox, {
                    label: "Enable Dispatch Telemetry",
                    checked: s,
                    onChange: () => d(!s)
                }), (0,
                n.jsx)("div", {
                    className: l()(c && f.Mq),
                    children: (0,
                    n.jsx)(i.Checkbox, {
                        label: "Show Dispatch Scheduler Telemetry",
                        checked: c,
                        onChange: () => {
                            u(e => {
                                let t = !e;
                                return t && d(!0),
                                t
                            }
                            )
                        }
                    })
                }), c ? (0,
                n.jsxs)(n.Fragment, {
                    children: [(0,
                    n.jsx)(i.Text, {
                        variant: "text-md/medium",
                        color: "text-muted",
                        children: "Dispatch Scheduler Telemetry:"
                    }), (0,
                    n.jsx)("table", {
                        cellPadding: 4,
                        children: Object.entries(r.generateTelemetry()).map(e => {
                            let[t,r] = e;
                            return (0,
                            n.jsxs)("tr", {
                                children: [(0,
                                n.jsx)("td", {
                                    children: (0,
                                    n.jsx)(i.Text, {
                                        variant: "text-xs/normal",
                                        color: "text-default",
                                        children: t
                                    })
                                }), (0,
                                n.jsx)("td", {
                                    children: (0,
                                    n.jsx)(i.Text, {
                                        tag: "span",
                                        variant: "text-xs/bold",
                                        color: "text-default",
                                        children: r
                                    })
                                })]
                            }, t)
                        }
                        )
                    }), (0,
                    n.jsx)("div", {
                        className: f.m8,
                        children: (0,
                        n.jsx)(i.Button, {
                            variant: "primary",
                            size: "sm",
                            text: "Reset Scheduler Telemetry",
                            onClick: () => {
                                r.reset()
                            }
                        })
                    })]
                }) : null]
            })
        }
        function S() {
            let[,e] = a.useState({});
            a.useEffect( () => {
                let t = setInterval( () => {
                    e({})
                }
                , h);
                return () => {
                    clearInterval(t)
                }
            }
            , [])
        }
        function O() {
            let e = (0,
            u.bG)([d.A], () => d.A.getSocket())
              , [t,r] = a.useState(!1);
            return S(),
            (0,
            n.jsx)("div", {
                className: l()(m.nd, f.nd),
                children: (0,
                n.jsxs)(i.IpV, {
                    className: f.nd,
                    children: [(0,
                    n.jsx)(y, {
                        socket: e,
                        isAverageFrameTime: t
                    }), (0,
                    n.jsx)(T, {
                        socket: e,
                        isAverageFrameTime: t,
                        onToggleAverageFrameTime: r
                    }), (0,
                    n.jsx)(R, {
                        socket: e
                    }), (0,
                    n.jsx)(C, {
                        socket: e
                    })]
                })
            })
        }
    },
    379078(e, t, r) {
        r.d(t, {
            n: () => c,
            r: () => l
        });
        var n, a, c = ((n = {}).FUZZY = "fuzzy",
        n.EXACT = "exact",
        n.REGEX = "regex",
        n.JARO_WINKLER = "jaro_winkler",
        n), l = ((a = {}).NONE = "none",
        a.JARO_WINKLER = "jaro_winkler",
        a)
    },
    704554(e, t, r) {
        r.d(t, {
            RT: () => o
        }),
        r(896048),
        r(693327),
        r(554719),
        r(680155),
        r(323874),
        r(14289),
        r(35956),
        r(733351);
        var n = r(64700)
          , a = r(812729)
          , c = r.n(a)
          , l = r(735438)
          , u = r(403362)
          , s = r(379078);
        let i = new Worker(new URL("/assets/" + r.u("83450"),r.b));
        function o(e, t, r, a) {
            let o = arguments.length > 4 && void 0 !== arguments[4] ? arguments[4] : []
              , d = n.useRef(null)
              , f = n.useRef(null)
              , m = n.useRef(r)
              , {searchStringGenerator: x} = a
              , h = function(e) {
                let t = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : []
                  , [r,a] = n.useState(e)
                  , l = n.useRef(e);
                return n.useEffect( () => {
                    l.current = e
                }
                , [e]),
                n.useEffect( () => {
                    a(e => {
                        let t = l.current;
                        return c()(e, t) ? e : t
                    }
                    )
                }
                , t),
                r
            }(t.map(x), [t, x, ...o])
              , b = function(e) {
                let t = arguments.length > 1 && void 0 !== arguments[1] ? arguments[1] : []
                  , [r,a] = n.useState(e)
                  , l = n.useRef(e);
                return n.useEffect( () => {
                    l.current = e
                }
                , [e]),
                n.useEffect( () => {
                    a(e => {
                        let t = l.current;
                        return c()(e, t) ? e : t
                    }
                    )
                }
                , t),
                r
            }(t, [t]);
            n.useEffect( () => {
                m.current = r
            }
            , [r]);
            let g = n.useMemo( () => {
                let {throttleMs: e=200, throttleLeading: t=!0, throttleTrailing: r=!0, maxSearchResults: n=-1} = a;
                return f.current = (0,
                l.throttle)(async (e, t, r) => {
                    if ("" === e.trim())
                        return void (n > 0 ? m.current(t.slice(0, n)) : m.current(t));
                    d.current = (0,
                    l.uniqueId)();
                    let c = await function(e, t, r, n) {
                        var a;
                        let c = null != (a = n.promiseUuid) ? a : (0,
                        l.uniqueId)()
                          , {searchType: o=s.n.FUZZY, sortType: d=s.r.NONE, jaroWinklerSearchThreshold: f=.85, maxSearchResults: m=-1} = n;
                        return new Promise(n => {
                            let a = t => {
                                let {data: {id: r, foundItemIndexes: l}} = t;
                                c === r && (n(l.map(t => e[t]).filter(u.Vq)),
                                null == i || i.removeEventListener("message", a))
                            }
                            ;
                            null == i || i.addEventListener("message", a),
                            null == i || i.postMessage({
                                id: c,
                                searchTerm: t,
                                searchStrings: r,
                                searchType: o,
                                sortType: d,
                                jaroWinklerSearchThreshold: f,
                                maxSearchResults: m
                            })
                        }
                        )
                    }(t, e, r, function(e) {
                        for (var t = 1; t < arguments.length; t++) {
                            var r = null != arguments[t] ? arguments[t] : {}
                              , n = Object.keys(r);
                            "function" == typeof Object.getOwnPropertySymbols && (n = n.concat(Object.getOwnPropertySymbols(r).filter(function(e) {
                                return Object.getOwnPropertyDescriptor(r, e).enumerable
                            }))),
                            n.forEach(function(t) {
                                var n;
                                n = r[t],
                                t in e ? Object.defineProperty(e, t, {
                                    value: n,
                                    enumerable: !0,
                                    configurable: !0,
                                    writable: !0
                                }) : e[t] = n
                            })
                        }
                        return e
                    }({
                        promiseUuid: d.current
                    }, a));
                    null != d.current && m.current(c)
                }
                , e, {
                    leading: t,
                    trailing: r
                }),
                f.current
            }
            , [a]);
            return n.useEffect( () => {
                g(e, b, h)
            }
            , [g, e, b, h, ...o]),
            n.useEffect( () => () => {
                null != f.current && f.current.cancel(),
                f.current = null,
                d.current = null
            }
            , [h, r, a]),
            g
        }
    }
}]);
//# sourceMappingURL=84ceaa92c203982c.js.map
