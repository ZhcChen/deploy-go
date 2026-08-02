import {
  AppWindow,
  Box,
  FileClock,
  Gauge,
  KeySquare,
  KeyRound,
  UserRound,
  Server,
  Settings,
  Users,
  type LucideIcon,
} from "lucide-react";

export interface RouteMetadata {
  path: string;
  label: string;
  title: string;
  icon: LucideIcon;
  section?: "settings";
}

export const primaryRoutes: RouteMetadata[] = [
  { path: "/overview", label: "概览", title: "概览", icon: Gauge },
  { path: "/deployments", label: "部署", title: "部署", icon: AppWindow },
  { path: "/apps", label: "应用", title: "应用", icon: Box },
  { path: "/nodes", label: "节点", title: "节点", icon: Server },
  { path: "/profile", label: "我的", title: "我的", icon: UserRound },
  { path: "/settings", label: "设置", title: "系统设置", icon: Settings },
];

export const settingsRoutes: RouteMetadata[] = [
  {
    path: "/settings",
    label: "系统设置",
    title: "系统设置",
    icon: Settings,
    section: "settings",
  },
  {
    path: "/settings/users",
    label: "用户管理",
    title: "用户管理",
    icon: Users,
    section: "settings",
  },
  {
    path: "/settings/credentials",
    label: "SSH 密钥",
    title: "SSH 密钥",
    icon: KeyRound,
    section: "settings",
  },
  {
    path: "/settings/application-access",
    label: "应用授权",
    title: "应用授权",
    icon: KeySquare,
    section: "settings",
  },
  {
    path: "/settings/audit",
    label: "审计记录",
    title: "审计记录",
    icon: FileClock,
    section: "settings",
  },
];

export function metadataFor(pathname: string) {
  return [...settingsRoutes, ...primaryRoutes]
    .sort((left, right) => right.path.length - left.path.length)
    .find(
      (route) =>
        pathname === route.path || pathname.startsWith(`${route.path}/`),
    );
}
