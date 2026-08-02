export interface ErrorNoticeValue {
  message: string;
  requestId?: string;
}

export function ApiErrorNotice({ error }: { error: ErrorNoticeValue }) {
  return (
    <div className="notice notice--danger" role="alert">
      <strong>{error.message}</strong>
      {error.requestId ? <small>Request ID: {error.requestId}</small> : null}
    </div>
  );
}
