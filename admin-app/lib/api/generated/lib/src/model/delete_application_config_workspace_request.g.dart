// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'delete_application_config_workspace_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeleteApplicationConfigWorkspaceRequest
    extends DeleteApplicationConfigWorkspaceRequest {
  @override
  final String bindingId;

  factory _$DeleteApplicationConfigWorkspaceRequest([
    void Function(DeleteApplicationConfigWorkspaceRequestBuilder)? updates,
  ]) => (DeleteApplicationConfigWorkspaceRequestBuilder()..update(updates))
      ._build();

  _$DeleteApplicationConfigWorkspaceRequest._({required this.bindingId})
    : super._();
  @override
  DeleteApplicationConfigWorkspaceRequest rebuild(
    void Function(DeleteApplicationConfigWorkspaceRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  DeleteApplicationConfigWorkspaceRequestBuilder toBuilder() =>
      DeleteApplicationConfigWorkspaceRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeleteApplicationConfigWorkspaceRequest &&
        bindingId == other.bindingId;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, bindingId.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'DeleteApplicationConfigWorkspaceRequest',
    )..add('bindingId', bindingId)).toString();
  }
}

class DeleteApplicationConfigWorkspaceRequestBuilder
    implements
        Builder<
          DeleteApplicationConfigWorkspaceRequest,
          DeleteApplicationConfigWorkspaceRequestBuilder
        > {
  _$DeleteApplicationConfigWorkspaceRequest? _$v;

  String? _bindingId;
  String? get bindingId => _$this._bindingId;
  set bindingId(String? bindingId) => _$this._bindingId = bindingId;

  DeleteApplicationConfigWorkspaceRequestBuilder() {
    DeleteApplicationConfigWorkspaceRequest._defaults(this);
  }

  DeleteApplicationConfigWorkspaceRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _bindingId = $v.bindingId;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeleteApplicationConfigWorkspaceRequest other) {
    _$v = other as _$DeleteApplicationConfigWorkspaceRequest;
  }

  @override
  void update(
    void Function(DeleteApplicationConfigWorkspaceRequestBuilder)? updates,
  ) {
    if (updates != null) updates(this);
  }

  @override
  DeleteApplicationConfigWorkspaceRequest build() => _build();

  _$DeleteApplicationConfigWorkspaceRequest _build() {
    final _$result =
        _$v ??
        _$DeleteApplicationConfigWorkspaceRequest._(
          bindingId: BuiltValueNullFieldError.checkNotNull(
            bindingId,
            r'DeleteApplicationConfigWorkspaceRequest',
            'bindingId',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
