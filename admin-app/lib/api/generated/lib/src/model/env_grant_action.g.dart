// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'env_grant_action.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const EnvGrantAction _$readWrite = const EnvGrantAction._('readWrite');
const EnvGrantAction _$delete = const EnvGrantAction._('delete');

EnvGrantAction _$valueOf(String name) {
  switch (name) {
    case 'readWrite':
      return _$readWrite;
    case 'delete':
      return _$delete;
    default:
      throw ArgumentError(name);
  }
}

final BuiltSet<EnvGrantAction> _$values = BuiltSet<EnvGrantAction>(
  const <EnvGrantAction>[_$readWrite, _$delete],
);

class _$EnvGrantActionMeta {
  const _$EnvGrantActionMeta();
  EnvGrantAction get readWrite => _$readWrite;
  EnvGrantAction get delete => _$delete;
  EnvGrantAction valueOf(String name) => _$valueOf(name);
  BuiltSet<EnvGrantAction> get values => _$values;
}

mixin _$EnvGrantActionMixin {
  // ignore: non_constant_identifier_names
  _$EnvGrantActionMeta get EnvGrantAction => const _$EnvGrantActionMeta();
}

Serializer<EnvGrantAction> _$envGrantActionSerializer =
    _$EnvGrantActionSerializer();

class _$EnvGrantActionSerializer
    implements PrimitiveSerializer<EnvGrantAction> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'readWrite': 'read_write',
    'delete': 'delete',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'read_write': 'readWrite',
    'delete': 'delete',
  };

  @override
  final Iterable<Type> types = const <Type>[EnvGrantAction];
  @override
  final String wireName = 'EnvGrantAction';

  @override
  Object serialize(
    Serializers serializers,
    EnvGrantAction object, {
    FullType specifiedType = FullType.unspecified,
  }) => _toWire[object.name] ?? object.name;

  @override
  EnvGrantAction deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) => EnvGrantAction.valueOf(
    _fromWire[serialized] ?? (serialized is String ? serialized : ''),
  );
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
