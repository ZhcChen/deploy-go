import { useQuery } from "@tanstack/react-query";
import { FileCog } from "lucide-react";
import { Link } from "react-router-dom";
import { PageState } from "../../components/PageState";
import { ApiErrorNotice } from "../errors/ApiErrorNotice";
import { toNotice } from "../shared/toNotice";
import { applicationConfigsApi } from "./api";

export function ApplicationConfigSection({
  applicationId,
  isAdministrator,
}: {
  applicationId: string;
  isAdministrator: boolean;
}) {
  const files = useQuery({
    queryKey: ["application-config-files", applicationId],
    queryFn: () => applicationConfigsApi.applicationConfigsList({ applicationId }),
  });
  const items = files.data?.items ?? [];
  const editableCount = items.filter((file) => file.editable).length;
  const incompleteCount = items.filter((file) => file.status === "incomplete").length;
  return (
    <section className="detail-section">
      <div className="section-heading">
        <div>
          <h3>应用配置副本</h3>
          <p>模板克隆出的可编辑配置副本；保存后生成不可变版本，部署 preview/confirm 会固化版本摘要。</p>
        </div>
        <div className="section-actions">
          {isAdministrator ? (
            <Link className="button button--primary" to={`/apps/${applicationId}/config`}>
              <FileCog aria-hidden="true" />
              打开配置工作区
            </Link>
          ) : null}
        </div>
      </div>
      {files.isLoading ? (
        <PageState kind="loading" />
      ) : files.isError ? (
        <ApiErrorNotice error={toNotice(files.error)} />
      ) : items.length === 0 ? (
        <p className="notice">该应用尚未从模板克隆配置副本。镜像目标可先初始化配置工作区，再从模板创建新应用。</p>
      ) : (
        <ul className="config-summary-list">
          <li><strong>{items.length}</strong><span>配置文件</span></li>
          <li><strong>{editableCount}</strong><span>可编辑</span></li>
          <li className={incompleteCount > 0 ? "config-summary-incomplete" : ""}><strong>{incompleteCount}</strong><span>待替换占位值</span></li>
        </ul>
      )}
    </section>
  );
}
