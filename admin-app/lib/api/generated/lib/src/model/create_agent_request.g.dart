// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'create_agent_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$CreateAgentRequest extends CreateAgentRequest {
  @override
  final String name;

  factory _$CreateAgentRequest([
    void Function(CreateAgentRequestBuilder)? updates,
  ]) => (CreateAgentRequestBuilder()..update(updates))._build();

  _$CreateAgentRequest._({required this.name}) : super._();
  @override
  CreateAgentRequest rebuild(
    void Function(CreateAgentRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  CreateAgentRequestBuilder toBuilder() =>
      CreateAgentRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is CreateAgentRequest && name == other.name;
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
      r'CreateAgentRequest',
    )..add('name', name)).toString();
  }
}

class CreateAgentRequestBuilder
    implements Builder<CreateAgentRequest, CreateAgentRequestBuilder> {
  _$CreateAgentRequest? _$v;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  CreateAgentRequestBuilder() {
    CreateAgentRequest._defaults(this);
  }

  CreateAgentRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(CreateAgentRequest other) {
    _$v = other as _$CreateAgentRequest;
  }

  @override
  void update(void Function(CreateAgentRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  CreateAgentRequest build() => _build();

  _$CreateAgentRequest _build() {
    final _$result =
        _$v ??
        _$CreateAgentRequest._(
          name: BuiltValueNullFieldError.checkNotNull(
            name,
            r'CreateAgentRequest',
            'name',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
