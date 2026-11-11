import { useEffect } from "react";
import { useBlocker } from "react-router-dom";

const message = "当前页面有未保存的修改，确定离开吗？";

/**
 * 有未保存的修改时，在应用内切换页面前请用户确认。
 *
 * 只拦截应用内的路由切换。关闭或刷新标签页、跳到其他网站都不拦截：不注册 beforeunload，
 * 避免浏览器弹出原生的“离开此网站？”确认。
 */
export function useUnsavedChanges(dirty: boolean) {
  const blocker = useBlocker(dirty);

  useEffect(() => {
    if (blocker.state !== "blocked") return;
    if (window.confirm(message)) blocker.proceed();
    else blocker.reset();
  }, [blocker]);
}
