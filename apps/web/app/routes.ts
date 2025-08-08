import {
  type RouteConfig,
  index,
  layout,
  route,
} from "@react-router/dev/routes";

//   { index: true, element: <Dashboard /> },
//   { path: 'dashboard', element: <Dashboard /> },
//   { path: 'sessions', element: <SessionsList /> },
//   { path: 'sessions/new', element: <NewSession /> },
//   { path: 'sessions/:id', element: <SessionDetail /> },
//   { path: 'topics', element: <Topics /> },
//   { path: 'settings', element: <Settings /> },

export default [
  index("./routes/home.tsx"),
  layout("./layouts/app-shell.tsx", [
    route("dashboard", "./routes/dashboard.tsx"),
    route("sessions", "./routes/sessions-list.tsx"),
    route("sessions/:id", "./routes/session-detail.tsx"),
    route("sessions/new", "./routes/new-session.tsx"),
    route("settings", "./routes/settings.tsx"),
    route("topics", "./routes/topics.tsx"),
  ]),
] satisfies RouteConfig;
