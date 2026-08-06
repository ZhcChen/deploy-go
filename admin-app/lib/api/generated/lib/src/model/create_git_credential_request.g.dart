// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_git_credential_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateGitCredentialRequest extends CreateGitCredentialRequest {
  @override
  final String name;

  factory _$CreateGitCredentialRequest([
    void Function(CreateGitCredentialRequestBuilder)? updates,
  ]) => (CreateGitCredentialRequestBuilder()..update(updates))._build();

  _$CreateGitCredentialRequest._({required this.name}) : super._();
  @override
  CreateGitCredentialRequest rebuild(
    void Function(CreateGitCredentialRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  CreateGitCredentialRequestBuilder toBuilder() =>
      CreateGitCredentialRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateGitCredentialRequest && name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'CreateGitCredentialRequest',
    )..add('name', name)).toString();
  }
}

class CreateGitCredentialRequestBuilder
    implements
        Builder<CreateGitCredentialRequest, CreateGitCredentialRequestBuilder> {
  _$CreateGitCredentialRequest? _$v;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  CreateGitCredentialRequestBuilder() {
    CreateGitCredentialRequest._defaults(this);
  }

  CreateGitCredentialRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateGitCredentialRequest other) {
    _$v = other as _$CreateGitCredentialRequest;
  }

  @override
  void update(void Function(CreateGitCredentialRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateGitCredentialRequest build() => _build();

  _$CreateGitCredentialRequest _build() {
    final _$result =
        _$v ??
        _$CreateGitCredentialRequest._(
          name: BuiltValueNullFieldError.checkNotNull(
            name,
            r'CreateGitCredentialRequest',
            'name',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
