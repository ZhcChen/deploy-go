import { useParams } from "react-router-dom";
import { BackLink } from "../../components/BackLink";
import { ApplicationConfigWorkspace } from "./ApplicationConfigWorkspace";

export function ApplicationConfigWorkspacePage() {
  const { id = "" } = useParams();
  return (
    <section className="workspace config-workspace-page">
      <BackLink to={`/apps/${id}`} parentLabel="应用" />
      <ApplicationConfigWorkspace applicationId={id} />
    </section>
  );
}
