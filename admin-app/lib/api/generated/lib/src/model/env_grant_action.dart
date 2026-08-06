//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'env_grant_action.g.dart';

class EnvGrantAction extends EnumClass {

  @BuiltValueEnumConst(wireName: r'read_write')
  static const EnvGrantAction readWrite = _$readWrite;
  @BuiltValueEnumConst(wireName: r'delete')
  static const EnvGrantAction delete = _$delete;

  static Serializer<EnvGrantAction> get serializer => _$envGrantActionSerializer;

  const EnvGrantAction._(String name): super(name);

  static BuiltSet<EnvGrantAction> get values => _$values;
  static EnvGrantAction valueOf(String name) => _$valueOf(name);
}

/// Optionally, enum_class can generate a mixin to go with your enum for use
/// with Angular. It exposes your enum constants as getters. So, if you mix it
/// in to your Dart component class, the values become available to the
/// corresponding Angular template.
///
/// Trigger mixin generation by writing a line like this one next to your enum.
abstract class EnvGrantActionMixin = Object with _$EnvGrantActionMixin;
