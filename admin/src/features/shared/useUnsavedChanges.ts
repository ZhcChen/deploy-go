import { useCallback, useEffect } from "react";
import { useBeforeUnload, useBlocker } from "react-router-dom";

const message = "当前页面有未保存的修改，确定离开吗？";

export function useUnsavedChanges(dirty: boolean) {
  const blocker = useBlocker(dirty);

  useBeforeUnload(useCallback((event) => {
    if (!dirty) return;
    event.preventDefault();
    event.returnValue = message;
  }, [dirty]));

  useEffect(() => {
    if (blocker.state !== "blocked") return;
    if (window.confirm(message)) blocker.proceed();
    else blocker.reset();
  }, [blocker]);
}
