// Webpack Module 183555
//EXTRACTED WEBPACK MODULE 183555
0,
function(e, t, r) {
    r.d(t, {
        pb: () => d
    });
    var n, u = r(582128), s = r(132500);
    let o = u.createContext(null);
    function d(e) {
        let {layout:t, userId:l, guildId:i, channelId:c, messageId:a, roleId:d, sourceSessionId:f, showGuildProfile:k=!0} = e
          , p = (n || (n = r.t(u, 2))).useContext(o)?.sessionId;
        return u.useMemo(() => ({
            sessionId: (0,
            s.A)(),
            sourceSessionId: f ?? p,
            layout: t,
            userId: l,
            guildId: i,
            channelId: c,
            messageId: a,
            roleId: d,
            showGuildProfile: k
        }), [p, t, l, i, c, a, d, f, k])
    }
}
