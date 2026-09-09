import { Check, Moon, Sun } from "lucide-react";
import { useTheme, type ThemeMode } from "../../theme/ThemeContext";

const themeOptions: Array<{
  value: ThemeMode;
  label: string;
  description: string;
  icon: typeof Moon;
}> = [
  {
    value: "dark",
    label: "暗色",
    description: "适合弱光环境，也是当前控制面的默认主题。",
    icon: Moon,
  },
  {
    value: "light",
    label: "亮色",
    description: "适合明亮环境，使用浅色背景和深色文字。",
    icon: Sun,
  },
];

export function AppearanceSettingsPage() {
  const { theme, setTheme } = useTheme();

  return (
    <section className="workspace settings-page">
      <div className="workspace-heading">
        <div>
          <h2>显示设置</h2>
          <p>选择控制面的配色主题，偏好保存在当前浏览器。</p>
        </div>
      </div>
      <section className="display-settings-section">
        <div className="section-heading">
          <div>
            <h3>主题模式</h3>
            <p>切换后立即生效，不会影响其他用户。</p>
          </div>
        </div>
        <div className="theme-choice-grid" role="radiogroup" aria-label="主题模式">
          {themeOptions.map(({ value, label, description, icon: Icon }) => {
            const selected = theme === value;
            return (
              <label className={`theme-option${selected ? " is-selected" : ""}`} key={value}>
                <input
                  type="radio"
                  name="theme"
                  value={value}
                  checked={selected}
                  aria-label={label}
                  onChange={() => setTheme(value)}
                />
                <span
                  className={`theme-option__preview theme-option__preview--${value}`}
                  aria-hidden="true"
                >
                  <span className="theme-option__preview-sidebar" />
                  <span className="theme-option__preview-body">
                    <i />
                    <i />
                    <i />
                  </span>
                </span>
                <span className="theme-option__body">
                  <span className="theme-option__title">
                    <Icon aria-hidden="true" />
                    {label}
                  </span>
                  <small>{description}</small>
                </span>
                <Check className="theme-option__check" aria-hidden="true" />
              </label>
            );
          })}
        </div>
        <p className="form-help theme-settings-note">
          代码编辑器、终端和日志查看器为了保持可读性，会继续使用深色配色。
        </p>
      </section>
    </section>
  );
}
