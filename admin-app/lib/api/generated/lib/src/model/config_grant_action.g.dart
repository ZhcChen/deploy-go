// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'config_grant_action.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const ConfigGrantAction _$readWrite = const ConfigGrantAction._('readWrite');

ConfigGrantAction _$valueOf(String name) {
  switch (name) {
    case 'readWrite':
      return _$readWrite;
    default:
      throw ArgumentError(name);
  }
}

final BuiltSet<ConfigGrantAction> _$values = BuiltSet<ConfigGrantAction>(
  const <ConfigGrantAction>[_$readWrite],
);

class _$ConfigGrantActionMeta {
  const _$ConfigGrantActionMeta();
  ConfigGrantAction get readWrite => _$readWrite;
  ConfigGrantAction valueOf(String name) => _$valueOf(name);
  BuiltSet<ConfigGrantAction> get values => _$values;
}

mixin _$ConfigGrantActionMixin {
  // ignore: non_constant_identifier_names
  _$ConfigGrantActionMeta get ConfigGrantAction =>
      const _$ConfigGrantActionMeta();
}

Serializer<ConfigGrantAction> _$configGrantActionSerializer =
    _$ConfigGrantActionSerializer();

class _$ConfigGrantActionSerializer
    implements PrimitiveSerializer<ConfigGrantAction> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'readWrite': 'read_write',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'read_write': 'readWrite',
  };

  @override
  final Iterable<Type> types = const <Type>[ConfigGrantAction];
  @override
  final String wireName = 'ConfigGrantAction';

  @override
  Object serialize(
    Serializers serializers,
    ConfigGrantAction object, {
    FullType specifiedType = FullType.unspecified,
  }) => _toWire[object.name] ?? object.name;

  @override
  ConfigGrantAction deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) => ConfigGrantAction.valueOf(
    _fromWire[serialized] ?? (serialized is String ? serialized : ''),
  );
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
