// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'register_application_envs_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RegisterApplicationEnvsResponse
    extends RegisterApplicationEnvsResponse {
  @override
  final BuiltList<String> created;
  @override
  final BuiltList<String> declared;

  factory _$RegisterApplicationEnvsResponse([
    void Function(RegisterApplicationEnvsResponseBuilder)? updates,
  ]) => (RegisterApplicationEnvsResponseBuilder()..update(updates))._build();

  _$RegisterApplicationEnvsResponse._({
    required this.created,
    required this.declared,
  }) : super._();
  @override
  RegisterApplicationEnvsResponse rebuild(
    void Function(RegisterApplicationEnvsResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RegisterApplicationEnvsResponseBuilder toBuilder() =>
      RegisterApplicationEnvsResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RegisterApplicationEnvsResponse &&
        created == other.created &&
        declared == other.declared;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, created.hashCode);
    _$hash = $jc(_$hash, declared.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RegisterApplicationEnvsResponse')
          ..add('created', created)
          ..add('declared', declared))
        .toString();
  }
}

class RegisterApplicationEnvsResponseBuilder
    implements
        Builder<
          RegisterApplicationEnvsResponse,
          RegisterApplicationEnvsResponseBuilder
        > {
  _$RegisterApplicationEnvsResponse? _$v;

  ListBuilder<String>? _created;
  ListBuilder<String> get created => _$this._created ??= ListBuilder<String>();
  set created(ListBuilder<String>? created) => _$this._created = created;

  ListBuilder<String>? _declared;
  ListBuilder<String> get declared =>
      _$this._declared ??= ListBuilder<String>();
  set declared(ListBuilder<String>? declared) => _$this._declared = declared;

  RegisterApplicationEnvsResponseBuilder() {
    RegisterApplicationEnvsResponse._defaults(this);
  }

  RegisterApplicationEnvsResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _created = $v.created.toBuilder();
      _declared = $v.declared.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RegisterApplicationEnvsResponse other) {
    _$v = other as _$RegisterApplicationEnvsResponse;
  }

  @override
  void update(void Function(RegisterApplicationEnvsResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RegisterApplicationEnvsResponse build() => _build();

  _$RegisterApplicationEnvsResponse _build() {
    _$RegisterApplicationEnvsResponse _$result;
    try {
      _$result =
          _$v ??
          _$RegisterApplicationEnvsResponse._(
            created: created.build(),
            declared: declared.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'created';
        created.build();
        _$failedField = 'declared';
        declared.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'RegisterApplicationEnvsResponse',
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
