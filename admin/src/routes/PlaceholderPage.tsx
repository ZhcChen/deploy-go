import { PageState } from "../components/PageState";

export function PlaceholderPage({ label }: { label: string }) {
  return (
    <section className="workspace" aria-label={label}>
      <div className="workspace-heading">
        <h2>{label}</h2>
      </div>
      <PageState kind="loading" />
    </section>
  );
}
