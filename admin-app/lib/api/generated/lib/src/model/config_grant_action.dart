//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'config_grant_action.g.dart';

class ConfigGrantAction extends EnumClass {

  @BuiltValueEnumConst(wireName: r'read_write')
  static const ConfigGrantAction readWrite = _$readWrite;

  static Serializer<ConfigGrantAction> get serializer => _$configGrantActionSerializer;

  const ConfigGrantAction._(String name): super(name);

  static BuiltSet<ConfigGrantAction> get values => _$values;
  static ConfigGrantAction valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class ConfigGrantActionMixin = Object with _$ConfigGrantActionMixin;
