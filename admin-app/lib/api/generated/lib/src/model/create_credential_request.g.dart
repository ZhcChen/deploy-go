// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_credential_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateCredentialRequest extends CreateCredentialRequest {
  @override
  final String name;

  factory _$CreateCredentialRequest(
          [void Function(CreateCredentialRequestBuilder)? updates]) =>
      (CreateCredentialRequestBuilder()..update(updates))._build();

  _$CreateCredentialRequest._({required this.name}) : super._();
  @override
  CreateCredentialRequest rebuild(
          void Function(CreateCredentialRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  CreateCredentialRequestBuilder toBuilder() =>
      CreateCredentialRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateCredentialRequest && name == other.name;
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
    return (newBuiltValueToStringHelper(r'CreateCredentialRequest')
          ..add('name', name))
        .toString();
  }
}

class CreateCredentialRequestBuilder
    implements
        Builder<CreateCredentialRequest, CreateCredentialRequestBuilder> {
  _$CreateCredentialRequest? _$v;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  CreateCredentialRequestBuilder() {
    CreateCredentialRequest._defaults(this);
  }

  CreateCredentialRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateCredentialRequest other) {
    _$v = other as _$CreateCredentialRequest;
  }

  @override
  void update(void Function(CreateCredentialRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateCredentialRequest build() => _build();

  _$CreateCredentialRequest _build() {
    final _$result = _$v ??
        _$CreateCredentialRequest._(
          name: BuiltValueNullFieldError.checkNotNull(
              name, r'CreateCredentialRequest', 'name'),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
