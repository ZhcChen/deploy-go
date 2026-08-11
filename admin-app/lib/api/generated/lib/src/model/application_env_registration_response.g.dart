// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_env_registration_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationEnvRegistrationResponse
    extends ApplicationEnvRegistrationResponse {
  @override
  final BuiltList<String> created;

  factory _$ApplicationEnvRegistrationResponse([
    void Function(ApplicationEnvRegistrationResponseBuilder)? updates,
  ]) => (ApplicationEnvRegistrationResponseBuilder()..update(updates))._build();

  _$ApplicationEnvRegistrationResponse._({required this.created}) : super._();
  @override
  ApplicationEnvRegistrationResponse rebuild(
    void Function(ApplicationEnvRegistrationResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationEnvRegistrationResponseBuilder toBuilder() =>
      ApplicationEnvRegistrationResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationEnvRegistrationResponse &&
        created == other.created;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, created.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'ApplicationEnvRegistrationResponse',
    )..add('created', created)).toString();
  }
}

class ApplicationEnvRegistrationResponseBuilder
    implements
        Builder<
          ApplicationEnvRegistrationResponse,
          ApplicationEnvRegistrationResponseBuilder
        > {
  _$ApplicationEnvRegistrationResponse? _$v;

  ListBuilder<String>? _created;
  ListBuilder<String> get created => _$this._created ??= ListBuilder<String>();
  set created(ListBuilder<String>? created) => _$this._created = created;

  ApplicationEnvRegistrationResponseBuilder() {
    ApplicationEnvRegistrationResponse._defaults(this);
  }

  ApplicationEnvRegistrationResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _created = $v.created.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationEnvRegistrationResponse other) {
    _$v = other as _$ApplicationEnvRegistrationResponse;
  }

  @override
  void update(
    void Function(ApplicationEnvRegistrationResponseBuilder)? updates,
  ) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationEnvRegistrationResponse build() => _build();

  _$ApplicationEnvRegistrationResponse _build() {
    _$ApplicationEnvRegistrationResponse _$result;
    try {
      _$result =
          _$v ??
          _$ApplicationEnvRegistrationResponse._(created: created.build());
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'created';
        created.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ApplicationEnvRegistrationResponse',
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
