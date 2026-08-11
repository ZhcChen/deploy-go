// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'register_admin_application_envs_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RegisterAdminApplicationEnvsRequest
    extends RegisterAdminApplicationEnvsRequest {
  @override
  final BuiltList<RegisterAdminApplicationEnvContent> files;

  factory _$RegisterAdminApplicationEnvsRequest([
    void Function(RegisterAdminApplicationEnvsRequestBuilder)? updates,
  ]) =>
      (RegisterAdminApplicationEnvsRequestBuilder()..update(updates))._build();

  _$RegisterAdminApplicationEnvsRequest._({required this.files}) : super._();
  @override
  RegisterAdminApplicationEnvsRequest rebuild(
    void Function(RegisterAdminApplicationEnvsRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RegisterAdminApplicationEnvsRequestBuilder toBuilder() =>
      RegisterAdminApplicationEnvsRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RegisterAdminApplicationEnvsRequest && files == other.files;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, files.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'RegisterAdminApplicationEnvsRequest',
    )..add('files', files)).toString();
  }
}

class RegisterAdminApplicationEnvsRequestBuilder
    implements
        Builder<
          RegisterAdminApplicationEnvsRequest,
          RegisterAdminApplicationEnvsRequestBuilder
        > {
  _$RegisterAdminApplicationEnvsRequest? _$v;

  ListBuilder<RegisterAdminApplicationEnvContent>? _files;
  ListBuilder<RegisterAdminApplicationEnvContent> get files =>
      _$this._files ??= ListBuilder<RegisterAdminApplicationEnvContent>();
  set files(ListBuilder<RegisterAdminApplicationEnvContent>? files) =>
      _$this._files = files;

  RegisterAdminApplicationEnvsRequestBuilder() {
    RegisterAdminApplicationEnvsRequest._defaults(this);
  }

  RegisterAdminApplicationEnvsRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _files = $v.files.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RegisterAdminApplicationEnvsRequest other) {
    _$v = other as _$RegisterAdminApplicationEnvsRequest;
  }

  @override
  void update(
    void Function(RegisterAdminApplicationEnvsRequestBuilder)? updates,
  ) {
    if (updates != null) updates(this);
  }

  @override
  RegisterAdminApplicationEnvsRequest build() => _build();

  _$RegisterAdminApplicationEnvsRequest _build() {
    _$RegisterAdminApplicationEnvsRequest _$result;
    try {
      _$result =
          _$v ?? _$RegisterAdminApplicationEnvsRequest._(files: files.build());
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'files';
        files.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'RegisterAdminApplicationEnvsRequest',
          _$failedField,
          e.toString(),
        );
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
